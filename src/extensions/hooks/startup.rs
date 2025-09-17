//! Startup hooks for direct adding to your penrose config.
use crate::{
    core::{conn::Conn, hooks::StateHook, State},
    util::spawn,
    x::{Atom, Prop, XConn},
    Result,
};
use std::borrow::Cow;

/// Spawn a client program on window manager startup
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnOnStartup {
    prog: Cow<'static, str>,
}

impl SpawnOnStartup {
    /// Create a new unboxed startup hook ready for adding to your Config
    pub fn new(prog: impl Into<Cow<'static, str>>) -> Self {
        Self { prog: prog.into() }
    }

    /// Create a new startup hook ready for adding to your Config
    pub fn boxed<C>(prog: impl Into<Cow<'static, str>>) -> Box<dyn StateHook<C>>
    where
        C: Conn,
    {
        Box::new(Self::new(prog))
    }
}

impl<C> StateHook<C> for SpawnOnStartup
where
    C: Conn,
{
    fn call(&mut self, _state: &mut State<C>, _: &mut C) -> Result<()> {
        spawn(self.prog.as_ref())
    }
}

/// Remove _NET_WM_STATE_FULLSCREEN property from existing clients, not present in
/// [client_set][0], on window manager startup.
///
///   [0]: crate::core::State::client_set
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearFsPropOnStartup;

impl ClearFsPropOnStartup {
    /// Create a new startup hook ready for adding to your Config
    pub fn boxed<X>() -> Box<dyn StateHook<X>>
    where
        X: XConn,
    {
        Box::new(Self)
    }
}

impl<X> StateHook<X> for ClearFsPropOnStartup
where
    X: XConn,
{
    fn call(&mut self, state: &mut State<X>, x: &mut X) -> Result<()> {
        for id in x.existing_clients()? {
            if !state.client_set.contains(&id) && x.client_should_be_managed(id) {
                let net_wm_state = Atom::NetWmState.as_ref();
                let full_screen = x.intern_atom(Atom::NetWmStateFullscreen.as_ref())?;
                let mut wstate = match x.get_prop(id, net_wm_state) {
                    Ok(Some(Prop::Cardinal(vals))) => vals,
                    _ => vec![],
                };
                if wstate.contains(&full_screen) {
                    wstate.retain(|&val| val != *full_screen);
                    x.set_prop(id, net_wm_state, Prop::Cardinal(wstate))?;
                }
            }
        }
        Ok(())
    }
}
