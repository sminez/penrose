//! A tiling window manager in the style of Xmonad
// TODO: enable these once we are stable enough
// #![warn(missing_debug_implementations)]
// #![warn(missing_docs)]

#[macro_use]
extern crate log;

#[macro_use]
pub mod macros;

pub mod client;
pub mod data_types;
pub mod helpers;
pub mod layout;
pub mod manager;
pub mod screen;
pub mod workspace;
pub mod xconnection;

// top level re-exports
pub use data_types::{ColorScheme, Config};
pub use layout::{Layout, LayoutConf};
pub use manager::WindowManager;
pub use xconnection::XcbConnection;
