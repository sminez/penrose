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

#[cfg(feature = "draw")]
pub mod draw;

#[cfg(feature = "xcb_layer")]
pub mod xcb;

// top level re-exports
#[doc(inline)]
pub use crate::core::{
    config::Config,
    data_types::Change::*,
    manager::WindowManager,
    ring::{Direction::*, InsertPoint, Selector},
};

#[cfg(feature = "xcb_layer")]
#[doc(inline)]
pub use crate::xcb::{new_xcb_backed_window_manager, new_xcb_connection, XcbConnection};

/// A default 'anyhow' based result type
pub type Result<T> = anyhow::Result<T>;
