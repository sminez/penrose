//! A tiling window manager in the style of Xmonad
#![warn(missing_docs, rust_2018_idioms, broken_intra_doc_links)]
#![deny(clippy::all)]
#![allow(clippy::too_many_arguments)]
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
    data_types::{Change::*, Config},
    manager::WindowManager,
    ring::{Direction::*, InsertPoint, Selector},
};

#[cfg(feature = "xcb_layer")]
#[doc(inline)]
pub use crate::xcb::{new_xcb_backed_window_manager, new_xcb_connection, XcbConnection};

/// A default 'anyhow' based result type
pub type Result<T> = anyhow::Result<T>;
