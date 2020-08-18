//! A tiling window manager in the style of Xmonad
#![warn(missing_docs)]

#[macro_use]
extern crate log;

#[macro_use]
pub mod core;

pub mod contrib;

#[cfg(feature = "draw")]
pub mod draw;

// top level re-exports
pub use crate::core::client;
pub use crate::core::data_types;
pub use crate::core::helpers;
pub use crate::core::hooks;
pub use crate::core::layout;
pub use crate::core::manager;
pub use crate::core::screen;
pub use crate::core::workspace;
pub use crate::core::xconnection;

pub use data_types::{Change::*, Config, Direction::*};
pub use manager::WindowManager;
pub use xconnection::XcbConnection;

use anyhow;

/// A default 'anyhow' based result type
pub type Result<T> = anyhow::Result<T>;
