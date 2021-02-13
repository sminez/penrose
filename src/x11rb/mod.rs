//! Helpers and utilities for using x11rb as a back end for penrose
//!
//! Docs for the `X11` core protocol can be found [here][1]. x11rb is a thin facade over this. For
//! X11 extensions, there are usually separate documentations. For example, the RandR extension is
//! documented in [2].
//!
//! [1]: https://www.x.org/releases/X11R7.6/doc/xproto/x11protocol.html
//! [2]: https://gitlab.freedesktop.org/xorg/proto/randrproto/-/blob/master/randrproto.txt

use crate::core::xconnection::{XError, Xid};

use x11rb::errors::{ConnectError, ConnectionError, ReplyError, ReplyOrIdError};

pub(crate) mod atom;
pub mod xconn;

/// Result type for fallible methods using XCB
pub type Result<T> = std::result::Result<T, X11rbError>;

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
}

macro_rules! from_error {
    ($type:ident) => {
        impl From<$type> for XError {
            fn from(error: $type) -> Self {
                X11rbError::from(error).into()
            }
        }
    }
}

from_error!(ConnectError);
from_error!(ConnectionError);
from_error!(ReplyError);
from_error!(ReplyOrIdError);
