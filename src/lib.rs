//! A tiling window manager in the style of Xmonad
#![warn(
    missing_docs,
    // rust_2018_idioms,
    broken_intra_doc_links
)]
#![deny(clippy::all)]
#![allow(clippy::too_many_arguments)]

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
pub use crate::core::{
    bindings, client,
    data_types::{self, Change::*, Config},
    helpers, hooks, layout,
    manager::{self, WindowManager},
    ring::{Direction::*, InsertPoint, Selector},
    screen, workspace, xconnection,
};

#[cfg(feature = "xcb_layer")]
pub use crate::xcb::{new_xcb_connection, XcbConnection};

/// A default 'anyhow' based result type
pub type Result<T> = anyhow::Result<T>;
