//! Helpers and utilities for using x11rb as a back end for penrose

use crate::{
    core::{
        config::Config,
        data_types::WinId,
        hooks::Hook,
        manager::WindowManager,
        xconnection::Atom,
    },
    ErrorHandler,
};

use x11rb::rust_connection::RustConnection;

pub mod xconn;

#[doc(inline)]
pub use xconn::X11rbConnection;

/// Result type for fallible methods using x11rb
pub type Result<T> = std::result::Result<T, X11rbError>;

/// Construct a penrose [WindowManager] backed by the default [x11rb][crate::x11rb] backend.
pub fn new_x11rb_backed_window_manager(
    config: Config,
    hooks: Vec<Box<dyn Hook<X11rbConnection<RustConnection>>>>,
    error_handler: ErrorHandler,
) -> crate::Result<WindowManager<X11rbConnection<RustConnection>>> {
    let (inner_conn, _) = RustConnection::connect(None).map_err(|err| X11rbError::from(err))?;
    let conn = X11rbConnection::new_for_connection(inner_conn)?;
    let mut wm = WindowManager::new(config, conn, hooks, error_handler);
    wm.init()?;

    Ok(wm)
}

/// Enum to store the various ways that operations can fail inside of the
/// x11rb implementations of penrose traits.
#[derive(thiserror::Error, Debug)]
pub enum X11rbError {
    /// Unable to establish a connection to the X server
    #[error(transparent)]
    Connect(#[from] ::x11rb::errors::ConnectError),

    /// The X11 connection broke
    #[error(transparent)]
    Connection(#[from] ::x11rb::errors::ConnectionError),

    /// Could not get X11 request reply
    #[error(transparent)]
    ReplyError(#[from] ::x11rb::errors::ReplyError),

    /// Could not get X11 request reply or could not generate_id()
    #[error(transparent)]
    ReplyOrIdError(#[from] ::x11rb::errors::ReplyOrIdError),

    /// A requested client property was empty
    #[error("'{}' prop is not set for client {1}", .0.as_ref())]
    MissingProp(Atom, WinId),

    /// The X11 server does not support the RandR extension
    #[error("the X11 server does not support the RandR extension")]
    MissingRandRSupport,
}
