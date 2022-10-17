//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{actions::key_handler, bindings::KeyEventHandler, core::State, x::XConn};
use tracing::info;

/// Exit penrose
///
/// Immediately exit the window manager with exit code 0.
pub fn exit<X, E>() -> Box<dyn KeyEventHandler<X, E>>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    key_handler(|_, _| std::process::exit(0))
}

/// Info log the current window manager [State].
pub fn log_current_state<X, E>() -> Box<dyn KeyEventHandler<X, E>>
where
    X: XConn + std::fmt::Debug,
    E: std::fmt::Debug + Send + Sync + 'static,
{
    key_handler(|s: &mut State<X, E>, _| {
        info!("Current Window Manager State: {s:#?}");
        Ok(())
    })
}
