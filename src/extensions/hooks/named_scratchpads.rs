//! Support for managing multiple floating scratchpad programs that can be
//! toggled on or off on the active workspace.
use crate::{
    core::{bindings::KeyEventHandler, hooks::ManageHook, State, WindowManager},
    util::spawn,
    x::{Query, XConn, XConnExt},
    Result, Xid,
};
use std::{collections::HashMap, fmt};
use tracing::{debug, error, warn};

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
        run_hook_on_toggle: bool,
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

        (
            nsp,
            ToggleNamedScratchPad {
                name,
                run_hook_on_toggle,
            },
        )
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
        .add_invisible_workspace(NSP_TAG)
        .expect("named scratchpad tag to be unique");
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ToggleNamedScratchPad {
    name: &'static str,
    run_hook_on_toggle: bool,
}

impl<X: XConn + 'static> KeyEventHandler<X> for ToggleNamedScratchPad {
    #[tracing::instrument(level = "debug", skip(state, x))]
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        let _s = state.extension::<NamedScratchPadState<X>>()?;
        let mut s = _s.borrow_mut();
        let name = self.name;

        let (id, hook) = match s.0.get_mut(&name) {
            // Active client somewhere in the StackSet
            Some(NamedScratchPad {
                client: Some(id),
                hook,
                ..
            }) if state.client_set.contains(id) => (*id, hook),

            // No active client or client is no longer in state
            Some(nsp) => {
                debug!(%nsp.prog, %name, "spawning NamedScratchPad program");
                nsp.client = None;
                return spawn(nsp.prog);
            }

            // The user created a ToggleNamedScratchPad but didn't register the scratchpad
            None => {
                warn!(%name, "toggle called for unknown scratchpad: did you remember to call add_named_scratchpads?");
                return Ok(());
            }
        };

        debug!(
            current_tag = state.client_set.current_tag(),
            current_screen = state.client_set.current_screen().index(),
            "Toggling nsp client"
        );

        if state.client_set.current_workspace().contains(&id) {
            // Toggle off: hiding the client on our invisible workspace
            debug!("current workspace contains target client: moving to NSP tag");
            state.client_set.move_client_to_tag(&id, NSP_TAG);
        } else {
            // Toggle on / bring to current workspace
            debug!("current workspace does not contain target client: moving to tag");
            state.client_set.move_client_to_current_tag(&id);

            if self.run_hook_on_toggle {
                if let Err(e) = hook.call(id, state, x) {
                    error!(%e, %name, "unable to run NSP manage hook during toggle");
                }
            }
        }

        x.refresh(state)
    }
}
