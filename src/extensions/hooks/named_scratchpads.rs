//! Support for managing multiple floating scratchpad programs that can be
//! toggled on or off on the active workspace.
use crate::{
    bindings::KeyEventHandler,
    core::{State, WindowManager},
    hooks::ManageHook,
    util::spawn,
    x::{Query, XConn, XConnExt},
    Result, Xid,
};
use std::{collections::HashMap, fmt};

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
            .field("client", &self.client)
            .field("name", &self.name)
            .field("prog", &self.prog)
            .finish()
    }
}

impl<X> NamedScratchPad<X>
where
    X: XConn,
{
    pub fn new<Q, H>(name: &'static str, prog: &'static str, query: Q, manage_hook: H) -> Self
    where
        Q: Query<X> + 'static,
        H: ManageHook<X> + 'static,
    {
        Self {
            name,
            prog,
            client: None,
            query: Box::new(query),
            hook: Box::new(manage_hook),
        }
    }

    /// Get a [KeyEventHandler] for toggling the visibility of this NamedScratchPad.
    pub fn toggle_action(&self) -> ToggleNamedScratchPad {
        ToggleNamedScratchPad(self.name)
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

    // wm.state.config.compose_or_set_refresh_hook(EwhmRefreshHook);
    // wm.state.config.compose_or_set_event_hook(EwhmEventHook);
    wm.state.config.compose_or_set_manage_hook(manage_hook);

    wm
}

pub fn manage_hook<X: XConn + 'static>(id: Xid, state: &mut State<X>, x: &X) -> Result<()> {
    let s = state.extension::<NamedScratchPadState<X>>()?;

    for sp in s.borrow_mut().0.values_mut() {
        if sp.client.is_some() {
            continue;
        }

        if sp.query.run(id, x)? {
            sp.client = Some(id);
            return sp.hook.call(id, state, x);
        }
    }

    Ok(())
}

/// Toggle the visibility of a NamedScratchPad.
///
/// See [NamedScratchPad::toggle_action].
pub struct ToggleNamedScratchPad(&'static str);

impl<X: XConn + 'static> KeyEventHandler<X> for ToggleNamedScratchPad {
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        let _s = state.extension::<NamedScratchPadState<X>>()?;
        let s = _s.borrow_mut();

        let nsp = s.0.get(&self.0).expect(
            "to only be able to construct a ToggleNamedScratchPad for an existing NamedScratchPad",
        );

        match nsp.client.as_ref() {
            Some(&id) => x.modify_and_refresh(state, |cs| {
                if cs.is_visible(&id) {
                    cs.remove_client(&id);
                    // TODO: clear the id on unmap
                } else {
                    cs.insert(id);
                }
            }),
            None => spawn(nsp.prog),
        }
    }
}

// TODO: event and refresh hooks for handling unmap and layout
