//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{
    core::{actions::key_handler, bindings::KeyEventHandler, State},
    x::XConn,
};
use tracing::info;

pub mod floating;

/// Exit penrose
///
/// Immediately exit the window manager with exit code 0.
pub fn exit<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|_, _| std::process::exit(0))
}

/// Info log the current window manager [State] for debugging purposes.
pub fn log_current_state<X: XConn + std::fmt::Debug>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|s: &mut State<X>, _| {
        info!("Current Window Manager State: {s:#?}");
        Ok(())
    })
}
