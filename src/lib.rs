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
pub use crate::core::bindings;
pub use crate::core::client;
pub use crate::core::data_types;
pub use crate::core::helpers;
pub use crate::core::hooks;
pub use crate::core::layout;
pub use crate::core::manager;
pub use crate::core::screen;
pub use crate::core::workspace;
pub use crate::core::xconnection;

pub use crate::core::ring::{Direction::*, InsertPoint, Selector};
pub use data_types::{Change::*, Config};
pub use manager::WindowManager;

#[cfg(feature = "xcb_layer")]
pub use crate::xcb::xconn::XcbConnection;

/// A default 'anyhow' based result type
pub type Result<T> = anyhow::Result<T>;
