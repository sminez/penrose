//! Startup hooks for direct adding to your penrose config.
use crate::{
    core::{hooks::StateHook, State},
    util::spawn,
    x::XConn,
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
    pub fn boxed<X>(prog: impl Into<Cow<'static, str>>) -> Box<dyn StateHook<X>>
    where
        X: XConn,
    {
        Box::new(Self::new(prog))
    }
}

impl<X> StateHook<X> for SpawnOnStartup
where
    X: XConn,
{
    fn call(&mut self, _state: &mut State<X>, _x: &X) -> Result<()> {
        spawn(self.prog.as_ref())
    }
}
