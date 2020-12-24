//! Core functionality for the penrose window manager library
#[macro_use]
pub mod macros;

pub mod bindings;
pub mod client;
pub mod data_types;
pub mod helpers;
pub mod hooks;
pub mod layout;
pub mod manager;
pub mod ring;
pub mod screen;
pub mod workspace;
pub mod xconnection;

pub use bindings::{FireAndForget, MouseEventHandler};
pub use client::Client;
pub use data_types::Config;
pub use hooks::Hook;
pub use layout::Layout;
pub use manager::WindowManager;
pub use ring::Selector;
pub use screen::Screen;
pub use workspace::Workspace;
