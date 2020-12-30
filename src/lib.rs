//! A tiling window manager in the style of Xmonad
#![warn(
    broken_intra_doc_links,
    missing_debug_implementations,
    future_incompatible,
    missing_docs,
    // missing_doc_code_examples,
    rust_2018_idioms,
)]
#![warn(clippy::all)]
#![allow(clippy::too_many_arguments, clippy::clippy::borrowed_box)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg",
    issue_tracker_base_url = "https://github.com/sminez/penrose/issues/"
)]

#[macro_use]
extern crate log;

#[macro_use]
pub mod core;

pub mod contrib;
pub mod draw;

#[cfg(feature = "xcb")]
pub mod xcb;

// top level re-exports
#[doc(inline)]
pub use crate::core::{
    config::Config,
    data_types::{Change::*, WinId},
    manager::WindowManager,
    ring::{Direction::*, InsertPoint, Selector},
};

#[cfg(feature = "xcb")]
#[doc(inline)]
pub use crate::xcb::{new_xcb_backed_window_manager, new_xcb_connection, XcbConnection};

/// Enum to store the various ways that operations can fail in Penrose
#[derive(thiserror::Error, Debug)]
pub enum PenroseError {
    /// Something went wrong using the [draw] module.
    ///
    /// See [DrawError][crate::draw::DrawError] for variants.
    #[error(transparent)]
    Draw(#[from] crate::draw::DrawError),

    /// An [IO Error][std::io::Error] was encountered
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Attempting to construct a penrose data type from an int failed.
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// A generic error type for use in user code when needing to construct
    /// a simple [PenroseError].
    #[error("Unhandled error: {0}")]
    Raw(String),

    /// An attempt to spawn an external process failed
    #[error("unable to get stdout handle for child process: {0}")]
    SpawnProc(String),

    /// The requested split point for partitioning a [Region][core::data_types::Region]
    /// was out of bounds
    #[error("Region split is out of range: {0} >= {1}")]
    SplitError(u32, u32),

    /// Parsing an [Atom][core::xconnection::Atom] from a str failed.
    ///
    /// This happens when the atom name being requested is not a known atom.
    #[error(transparent)]
    Strum(#[from] strum::ParseError),

    /// A user specified key binding contained an invalid modifier key
    #[error("Unknown modifier key: {0}")]
    UnknownModifier(String),

    /// A user specified mouse binding contained an invalid button
    #[error("Unknown mouse button: {0}")]
    UnknownMouseButton(u8),

    /// Something went wrong using the [xcb] module.
    ///
    /// See [XcbError][crate::xcb::XcbError] for variants.
    #[cfg(feature = "xcb")]
    #[error(transparent)]
    Xcb(#[from] crate::xcb::XcbError),
}

/// Top level penrose Result type
pub type Result<T> = std::result::Result<T, PenroseError>;
