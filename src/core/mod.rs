//! Core functionality for the Penrose window manager library
//!
//! # Overview
//!
//! At a high level, Penrose works as a single threaded event loop that is driven by [X events][1]
//! emitted by your X server. For the most part, how those events are processed is an
//! implementation detail of the [WindowManager][2] struct, but there are multiple places that you
//! can provide your own additional (or in some cases, alternate) functionality if desired. In
//! terms of data structures and the overall architecture of Penrose, you will always have the
//! following:
//!
//! - A [WindowManager][2] struct. This is the main event handling logic and top level data structure
//!   that coordinates everything else in response to X events.
//! - An [XConn][3] impl. This trait defines the API used by the `WindowManager` to interact with
//!   the X server itself. A default [xcb][4] backed implementation is provided in the form of
//!   [XcbConnection][5]. This handles all of the low level interaction with the X server.
//! - An [error handler][6] for running custom top level error handling logic (a simple default
//!   handler is provided that simply logs the error at `ERROR` level and then continues operation)
//! - [Key][7] and [mouse][8] bindings for accepting user input and running `WindowManager` methods.
//!
//! It is also possible to add [hooks][9], and things such as a [status bar][10] which itself uses
//! the hook system for listening to internal `WindowManager` events, but these are not required in
//! any way.
//!
//! [1]: crate::core::xconnection::XEvent
//! [2]: crate::core::manager::WindowManager
//! [3]: crate::core::xconnection::XConn
//! [4]: https://xcb.freedesktop.org/
//! [5]: crate::xcb::XcbConnection
//! [6]: crate::ErrorHandler
//! [7]: crate::gen_keybindings
//! [8]: crate::gen_mousebindings
//! [9]: crate::core::hooks
//! [10]: crate::draw::bar
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
pub use bindings::{KeyEventHandler, MouseEventHandler};
#[doc(inline)]
pub use client::Client;
#[doc(inline)]
pub use config::Config;
#[doc(inline)]
pub use hooks::{Hook, HookName};
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
