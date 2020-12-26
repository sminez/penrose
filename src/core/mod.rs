//! Core functionality for the penrose window manager library
#[macro_use]
pub mod macros;

pub mod bindings;
pub mod client;
pub mod config;
pub mod data_types;
pub mod helpers;
pub mod hooks;
pub mod layout;
pub mod manager;
pub mod ring;
pub mod screen;
pub mod workspace;
pub mod xconnection;

#[doc(inline)]
pub use bindings::{FireAndForget, MouseEventHandler};
#[doc(inline)]
pub use client::Client;
#[doc(inline)]
pub use config::Config;
#[doc(inline)]
pub use hooks::Hook;
#[doc(inline)]
pub use layout::Layout;
#[doc(inline)]
pub use manager::WindowManager;
#[doc(inline)]
pub use ring::Selector;
#[doc(inline)]
pub use screen::Screen;
#[doc(inline)]
pub use workspace::Workspace;
