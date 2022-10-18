use crate::{core::State, hooks::StateHook, util::spawn, x::XConn, Result};

/// Spawn a client program on window manager startup
pub struct SpawnOnStartup {
    prog: &'static str,
}

impl SpawnOnStartup {
    /// Create a new startup hook ready for adding to your Config
    pub fn boxed<X, E>(prog: &'static str) -> Box<dyn StateHook<X, E>>
    where
        X: XConn,
        E: Send + Sync + 'static,
    {
        Box::new(Self { prog })
    }
}

impl<X, E> StateHook<X, E> for SpawnOnStartup
where
    X: XConn,
    E: Send + Sync + 'static,
{
    fn call(&mut self, _state: &mut State<X, E>, _x: &X) -> Result<()> {
        spawn(self.prog)
    }
}
