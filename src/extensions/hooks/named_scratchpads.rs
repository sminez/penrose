//! Support for managing multiple floating scratchpad programs that can be
//! toggled on or off on the active workspace.
use crate::{
    bindings::KeyEventHandler,
    core::{State, WindowManager},
    hooks::ManageHook,
    layout::LayoutStack,
    util::spawn,
    x::{Query, XConn, XConnExt},
    Result, Xid,
};
use std::{collections::HashMap, fmt};
use tracing::{debug, warn};

/// The tag used for a placeholder Workspace that holds scratchpad windows when
/// they are currently hidden.
pub const NSP_TAG: &str = "NSP";

/// A toggle-able client program that can be shown and hidden via a keybinding.
pub struct NamedScratchPad<X>
where
    X: XConn,
{
    name: &'static str,
    prog: &'static str,
    client: Option<Xid>,
    query: Box<dyn Query<X>>,
    hook: Box<dyn ManageHook<X>>,
}

impl<X: XConn> fmt::Debug for NamedScratchPad<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedScratchpad")
            .field("name", &self.name)
            .field("prog", &self.prog)
            .field("client", &self.client)
            .finish()
    }
}

impl<X> NamedScratchPad<X>
where
    X: XConn,
{
    pub fn new<Q, H>(
        name: &'static str,
        prog: &'static str,
        query: Q,
        manage_hook: H,
    ) -> (Self, ToggleNamedScratchPad)
    where
        Q: Query<X> + 'static,
        H: ManageHook<X> + 'static,
    {
        let nsp = Self {
            name,
            prog,
            client: None,
            query: Box::new(query),
            hook: Box::new(manage_hook),
        };

        (nsp, ToggleNamedScratchPad(name))
    }
}

// Private wrapper type to ensure that only this module can access this state extension
struct NamedScratchPadState<X: XConn>(HashMap<&'static str, NamedScratchPad<X>>);

/// Add the required hooks to manage EWMH compliance to an existing [Config].
///
/// See the module level docs for details of what functionality is provided by
/// this extension.
pub fn add_named_scratchpads<X>(
    mut wm: WindowManager<X>,
    scratchpads: Vec<NamedScratchPad<X>>,
) -> WindowManager<X>
where
    X: XConn + 'static,
{
    let state: HashMap<_, _> = scratchpads.into_iter().map(|nsp| (nsp.name, nsp)).collect();
    wm.state.add_extension(NamedScratchPadState(state));

    wm.state
        .client_set
        .add_invisible_workspace(NSP_TAG, LayoutStack::default());

    wm.state.config.compose_or_set_manage_hook(manage_hook);

    wm
}

/// Store clients matching NamedScratchPad queries and run the associated [ManageHook].
pub fn manage_hook<X: XConn + 'static>(id: Xid, state: &mut State<X>, x: &X) -> Result<()> {
    let s = state.extension::<NamedScratchPadState<X>>()?;

    for sp in s.borrow_mut().0.values_mut() {
        if sp.client.is_none() && sp.query.run(id, x)? {
            debug!(scratchpad=sp.name, %id, "matched query for named scratchpad");
            sp.client = Some(id);
            return sp.hook.call(id, state, x);
        }
    }

    Ok(())
}

/// Toggle the visibility of a NamedScratchPad.
///
/// This will spawn the requested client program if it isn't currently running or
/// move it to the focused workspace. If the scratchpad is currently visible it
/// will be hidden.
pub struct ToggleNamedScratchPad(&'static str);

impl<X: XConn + 'static> KeyEventHandler<X> for ToggleNamedScratchPad {
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        let s = state.extension::<NamedScratchPadState<X>>()?;
        let name = self.0;

        let id = match s.borrow_mut().0.get_mut(&name) {
            Some(nsp) => match nsp.client {
                Some(id) if state.client_set.contains(&id) => id,

                // No active client or client is no longer in state
                _ => {
                    debug!(%nsp.prog, %name, "spawning NamedScratchPad program");
                    nsp.client = None;
                    return spawn(nsp.prog);
                }
            },

            None => {
                warn!(%name, "toggle called for unknown scratchpad");
                return Ok(());
            }
        };

        x.modify_and_refresh(state, |cs| {
            if cs.current_workspace().contains(&id) {
                cs.move_client_to_tag(&id, NSP_TAG);
            } else {
                cs.move_client_to_current_tag(&id);
            }
        })
    }
}
