//! Helpers and utilities for using x11rb as a back end for penrose

use crate::core::{data_types::WinId, xconnection::Atom};

pub mod xconn;

/// Result type for fallible methods using x11rb
pub type Result<T> = std::result::Result<T, X11rbError>;

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
