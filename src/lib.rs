//! A tiling window manager in the style of Xmonad
// TODO: enable these once we are stable enough
// #![warn(missing_debug_implementations)]
// #![warn(missing_docs)]

#[macro_use]
extern crate log;

#[macro_use]
pub mod core;

pub mod contrib;

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
