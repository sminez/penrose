//! Helpers and utilities for using x11rb as a back end for penrose
//!
//! Docs for the `X11` core protocol can be found [here][1]. x11rb is a thin facade over this. For
//! X11 extensions, there are usually separate documentations. For example, the RandR extension is
//! documented in [2].
//!
//! [1]: https://www.x.org/releases/X11R7.6/doc/xproto/x11protocol.html
//! [2]: https://gitlab.freedesktop.org/xorg/proto/randrproto/-/blob/master/randrproto.txt

use crate::{
    core::{
        config::Config,
        hooks::{Hook, Hooks},
        manager::WindowManager,
        xconnection::{XError, Xid},
    },
    ErrorHandler,
};

#[cfg(feature = "x11rb-xcb")]
use x11rb::xcb_ffi::XCBConnection;
use x11rb::{
    connection::Connection,
    errors::{ConnectError, ConnectionError, ReplyError, ReplyOrIdError},
    rust_connection::RustConnection,
    x11_utils::X11Error,
};

pub(crate) mod atom;
pub(crate) mod event;
pub mod xconn;

#[doc(inline)]
pub use xconn::X11rbConnection;

/// Result type for fallible methods using x11rb
pub type Result<T> = std::result::Result<T, X11rbError>;

/// Helper type for when you are defining your [Hook] vector in your main.rs when using
/// the default x11rb impls
pub type X11rbHooks = Hooks<X11rbConnection<RustConnection>>;

/// Construct a penrose [WindowManager] backed by the [x11rb][crate::x11rb] backend using
/// [x11rb::rust_connection::RustConnection].
pub fn new_x11rb_rust_backed_window_manager(
    config: Config,
    hooks: Vec<Box<dyn Hook<X11rbConnection<RustConnection>>>>,
    error_handler: ErrorHandler,
) -> crate::Result<WindowManager<X11rbConnection<RustConnection>>> {
    let (conn, _) = RustConnection::connect(None).map_err(|err| X11rbError::from(err))?;
    new_x11rb_backed_window_manager(conn, config, hooks, error_handler)
}

/// Construct a penrose [WindowManager] backed by the [x11rb][crate::x11rb] backend using
/// [x11rb::xcb_ffi::XCBConnection].
#[cfg(feature = "x11rb-xcb")]
pub fn new_x11rb_xcb_backed_window_manager(
    config: Config,
    hooks: Vec<Box<dyn Hook<X11rbConnection<XCBConnection>>>>,
    error_handler: ErrorHandler,
) -> crate::Result<WindowManager<X11rbConnection<XCBConnection>>> {
    let (conn, _) = XCBConnection::connect(None).map_err(|err| X11rbError::from(err))?;
    new_x11rb_backed_window_manager(conn, config, hooks, error_handler)
}

/// Construct a penrose [WindowManager] backed by the [x11rb][crate::x11rb] backend using
/// the given connection.
pub fn new_x11rb_backed_window_manager<C: Connection>(
    connection: C,
    config: Config,
    hooks: Vec<Box<dyn Hook<X11rbConnection<C>>>>,
    error_handler: ErrorHandler,
) -> crate::Result<WindowManager<X11rbConnection<C>>> {
    let conn = X11rbConnection::new_for_connection(connection)?;
    let mut wm = WindowManager::new(config, conn, hooks, error_handler);
    wm.init()?;

    Ok(wm)
}

/// Enum to store the various ways that operations can fail inside of the
/// x11rb implementations of penrose traits.
#[derive(thiserror::Error, Debug)]
pub enum X11rbError {
    /// Unable to establish a connection to the X11 server
    #[error(transparent)]
    Connect(#[from] ConnectError),

    /// The X11 connection broke
    #[error(transparent)]
    Connection(#[from] ConnectionError),

    /// Could not get the reply to an X11 request
    #[error(transparent)]
    ReplyError(#[from] ReplyError),

    /// Could not get the reply to an X11 request or failed to allocate an Xid
    #[error(transparent)]
    ReplyOrIdError(#[from] ReplyOrIdError),

    /// A requested client property was empty
    #[error("'{0}' prop is not set for client {1}")]
    MissingProp(String, Xid),

    /// Something was not valid UTF8
    #[error("Something was expected to be UTF8, but is not")]
    NonUtf8(#[from] std::string::FromUtf8Error),

    /// Property data returned for the target window was in an invalid format
    #[error("invalid property data: {0}")]
    InvalidPropertyData(String),

    /// A query via the randr API was unsuccessful
    #[error("randr query failed: {0}")]
    Randr(String),

    /// Wrapper around low level X11 errors
    #[error("X11 error: {0:?}")]
    X11Error(X11Error),
}

macro_rules! from_error {
    ($type:ident) => {
        impl From<$type> for XError {
            fn from(error: $type) -> Self {
                X11rbError::from(error).into()
            }
        }
    };
}

from_error!(ConnectError);
from_error!(ConnectionError);
from_error!(ReplyError);
from_error!(ReplyOrIdError);
