//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{
    core::{
        bindings::{KeyEventHandler, MouseEventHandler},
        layout::IntoMessage,
        ClientSet, State,
    },
    util,
    x::{XConn, XConnExt},
    Result,
};
use tracing::info;

pub mod floating;

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

/// Mutate the [ClientSet] and refresh the on screen state
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

/// Exit penrose
///
/// Signal the `WindowManager` to exit it's main event loop.
pub fn exit<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|s: &mut State<X>, _| {
        s.running = false;
        Ok(())
    })
}

/// Info log the current window manager [State] for debugging purposes.
pub fn log_current_state<X: XConn + std::fmt::Debug>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|s: &mut State<X>, _| {
        info!("Current Window Manager State: {s:#?}");
        Ok(())
    })
}

/// Remove the currently focused client from state and unmap it WITHOUT
/// closing the client program.
/// This is provided for removing clients that have been accidentally tiled when
/// they should have been ignored.
pub fn remove_and_unmap_focused_client<X: XConn>() -> Box<dyn KeyEventHandler<X>> {
    key_handler(|s: &mut State<X>, x: &X| {
        if let Some(client) = s.client_set.remove_focused() {
            info!(
                ?client,
                "Unmapping previously focused client following removal from state"
            );
            x.unmap(client)
        } else {
            Ok(())
        }
    })
}

// NOTE: this is here to force the correct lifetime requirements on closures being
//       used as handlers. The generic impl in crate::bindings for functions of the
//       right signature isn't sufficient on its own.

/// Construct a [MouseEventHandler] from a closure or free function.
///
/// The resulting handler will run on button press events.
pub fn mouse_handler<F, X>(f: F) -> Box<dyn MouseEventHandler<X>>
where
    F: FnMut(&mut State<X>, &X) -> Result<()> + 'static,
    X: XConn,
{
    Box::new(f)
}

/// Mutate the [ClientSet] and refresh the on screen state
pub fn mouse_modify_with<F, X>(f: F) -> Box<dyn MouseEventHandler<X>>
where
    F: FnMut(&mut ClientSet) + Clone + 'static,
    X: XConn,
{
    Box::new(move |s: &mut State<X>, x: &X| x.modify_and_refresh(s, f.clone()))
}
