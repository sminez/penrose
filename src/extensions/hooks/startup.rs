//! Startup hooks for direct adding to your penrose config.
use crate::{
    core::{hooks::StateHook, State},
    util::spawn,
    x::XConn,
    Result,
};

/// Spawn a client program on window manager startup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOnStartup {
    prog: &'static str,
}

impl SpawnOnStartup {
    /// Create a new startup hook ready for adding to your Config
    pub fn boxed<X>(prog: &'static str) -> Box<dyn StateHook<X>>
    where
        X: XConn,
    {
        Box::new(Self { prog })
    }
}

impl<X> StateHook<X> for SpawnOnStartup
where
    X: XConn,
{
    fn call(&mut self, _state: &mut State<X>, _x: &X) -> Result<()> {
        spawn(self.prog)
    }
}
