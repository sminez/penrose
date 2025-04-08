//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{
    core::{
        bindings::KeyEventHandler,
        conn::{Conn, ConnExt},
        layout::IntoMessage,
        State,
    },
    pure::StackSet,
    util, Result, WinId,
};
use tracing::info;

pub mod floating;

// NOTE: this is here to force the correct lifetime requirements on closures being
//       used as handlers. The generic impl in crate::bindings for functions of the
//       right signature isn't sufficient on its own.

/// Construct a [KeyEventHandler] from a closure or free function
pub fn key_handler<F, C>(f: F) -> Box<dyn KeyEventHandler<C>>
where
    F: FnMut(&mut State<C>, &mut C) -> Result<()> + Send + 'static,
    C: Conn,
{
    Box::new(f)
}

/// Mutate the [StackSet<WinId>] and refresh the on screen state
pub fn modify_with<F, C>(f: F) -> Box<dyn KeyEventHandler<C>>
where
    F: FnMut(&mut StackSet<WinId>) + Clone + Send + 'static,
    C: Conn,
{
    Box::new(move |s: &mut State<C>, conn: &mut C| conn.modify_and_refresh(s, f.clone()))
}

/// Send a message to the currently active layout
pub fn send_layout_message<F, M, C>(f: F) -> Box<dyn KeyEventHandler<C>>
where
    F: Fn() -> M + Send + 'static,
    M: IntoMessage,
    C: Conn,
{
    key_handler(move |s: &mut State<C>, conn: &mut C| {
        conn.modify_and_refresh(s, |cs| {
            cs.current_workspace_mut().handle_message(f());
        })
    })
}

/// Send a message to all layouts available to the current workspace
pub fn broadcast_layout_message<F, M, C>(f: F) -> Box<dyn KeyEventHandler<C>>
where
    F: Fn() -> M + Send + 'static,
    M: IntoMessage,
    C: Conn,
{
    key_handler(move |s: &mut State<C>, conn: &mut C| {
        conn.modify_and_refresh(s, |cs| {
            cs.current_workspace_mut().broadcast_message(f());
        })
    })
}

/// Spawn an external program as part of a key binding
pub fn spawn<C>(program: &'static str) -> Box<dyn KeyEventHandler<C>>
where
    C: Conn,
{
    key_handler(move |_, _| util::spawn(program))
}

/// Exit penrose
///
/// Signal the `WindowManager` to exit it's main event loop.
pub fn exit<C: Conn>() -> Box<dyn KeyEventHandler<C>> {
    key_handler(|s: &mut State<C>, _| {
        s.running = false;
        Ok(())
    })
}

/// Info log the current window manager [State] for debugging purposes.
pub fn log_current_state<C: Conn + std::fmt::Debug>() -> Box<dyn KeyEventHandler<C>> {
    key_handler(|s: &mut State<C>, _| {
        info!("Current Window Manager State: {s:#?}");
        Ok(())
    })
}

/// Remove the currently focused client from state and unmap it WITHOUT
/// closing the client program.
/// This is provided for removing clients that have been accidentally tiled when
/// they should have been ignored.
pub fn remove_and_unmap_focused_client<C: Conn>() -> Box<dyn KeyEventHandler<C>> {
    key_handler(|s: &mut State<C>, conn: &mut C| {
        if let Some(client) = s.client_set.remove_focused() {
            info!(
                ?client,
                "Unmapping previously focused client following removal from state"
            );
            conn.hide_client(client)
        } else {
            Ok(())
        }
    })
}
