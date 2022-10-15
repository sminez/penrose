//! Helpers and pre-defined actions for use in user defined key bindings
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
pub fn key_handler<F, X, E>(f: F) -> Box<dyn KeyEventHandler<X, E>>
where
    F: FnMut(&mut State<X, E>, &X) -> Result<()> + 'static,
    X: XConn,
    E: Send + Sync + 'static,
{
    Box::new(f)
}

/// Mutate the [ClientSet] and refresh the onscreen state
pub fn modify_with<F, X, E>(f: F) -> Box<dyn KeyEventHandler<X, E>>
where
    F: FnMut(&mut ClientSet) + Clone + 'static,
    X: XConn,
    E: Send + Sync + 'static,
{
    Box::new(move |s: &mut State<X, E>, x: &X| x.modify_and_refresh(s, f.clone()))
}

/// Send a message to the currently active layout
pub fn send_layout_message<F, M, X, E>(f: F) -> Box<dyn KeyEventHandler<X, E>>
where
    F: Fn() -> M + 'static,
    M: IntoMessage,
    X: XConn,
    E: Send + Sync + 'static,
{
    key_handler(move |s: &mut State<X, E>, _| {
        s.client_set.current_workspace_mut().handle_message(f());

        Ok(())
    })
}

/// Send a message to all layouts available to the current workspace
pub fn broadcast_layout_message<F, M, X, E>(f: F) -> Box<dyn KeyEventHandler<X, E>>
where
    F: Fn() -> M + 'static,
    M: IntoMessage,
    X: XConn,
    E: Send + Sync + 'static,
{
    key_handler(move |s: &mut State<X, E>, _| {
        s.client_set.current_workspace_mut().broadcast_message(f());

        Ok(())
    })
}

/// Spawn an external program as part of a key binding
pub fn spawn<X, E>(program: &'static str) -> Box<dyn KeyEventHandler<X, E>>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    key_handler(move |_, _| util::spawn(program))
}

/// Exit penrose
pub fn exit<X, E>() -> Box<dyn KeyEventHandler<X, E>>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    key_handler(|_, _| std::process::exit(0))
}
