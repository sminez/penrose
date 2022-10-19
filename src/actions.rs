//! Helpers for writing user defined key bindings.
//!
//! See `penrose::extensions::actions` for pre-defined actions that are ready
//! for use.
use crate::{
    bindings::KeyEventHandler,
    core::{ClientSet, State},
    layout::IntoMessage,
    util,
    x::{XConn, XConnExt},
    Result,
};

// NOTE: this is here to force the correct lifetime requirements on closures being
//       used as handlers. The generic impl in crate::bindings for functions of the
//       right signature isn't sufficient on its own.

/// Construct a [KeyEventHandler] from a closure or free function
pub fn key_handler<F, X>(f: F) -> Box<dyn KeyEventHandler<X>>
where
    F: FnMut(&mut State<X>, &X) -> Result<()> + 'static,
    X: XConn,
{
    Box::new(f)
}

/// Mutate the [ClientSet] and refresh the onscreen state
pub fn modify_with<F, X>(f: F) -> Box<dyn KeyEventHandler<X>>
where
    F: FnMut(&mut ClientSet) + Clone + 'static,
    X: XConn,
{
    Box::new(move |s: &mut State<X>, x: &X| x.modify_and_refresh(s, f.clone()))
}

/// Send a message to the currently active layout
pub fn send_layout_message<F, M, X>(f: F) -> Box<dyn KeyEventHandler<X>>
where
    F: Fn() -> M + 'static,
    M: IntoMessage,
    X: XConn,
{
    key_handler(move |s: &mut State<X>, x: &X| {
        x.modify_and_refresh(s, |cs| {
            cs.current_workspace_mut().handle_message(f());
        })
    })
}

/// Send a message to all layouts available to the current workspace
pub fn broadcast_layout_message<F, M, X>(f: F) -> Box<dyn KeyEventHandler<X>>
where
    F: Fn() -> M + 'static,
    M: IntoMessage,
    X: XConn,
{
    key_handler(move |s: &mut State<X>, x: &X| {
        x.modify_and_refresh(s, |cs| {
            cs.current_workspace_mut().broadcast_message(f());
        })
    })
}

/// Spawn an external program as part of a key binding
pub fn spawn<X>(program: &'static str) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn,
{
    key_handler(move |_, _| util::spawn(program))
}
