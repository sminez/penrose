//! # Penrose: a library for building your very own tiling window manager
//!
//! Penrose is inspired by similar projects such as [dwm][1], [xmonad][2] and [qtile][3] which
//! allow you to configure your window manager in code and compile it for your system. It is most
//! similar to `xmonad` in that it is more of a library for building a window manager (with low
//! level details taken care of for you) rather than a minimal window manager that you edit and
//! patch directly (such as `dwm`). Penrose strives to be as simple as possible in its
//! implementation in order to make the guts of the window manager easier to understand. Given the
//! nature of what this involves, this is not always possible but effort has been made to make the
//! source readable and with relatively little magic.
//!
//! # Using Penrose
//!
//! Penrose itself is not a binary application that you can build, install and run. You need to
//! write your own **main.rs** as a rust binary crate that uses Penrose to set up, configure and
//! run your very own window manager exactly how you want it. In short, you *will* need to write
//! some code and you *will* need to know rust so some degree.
//!
//! For learning rust itself, there is some fantastic official [guides][4] available on
//! [rust-lang.org][15] and if you are sticking to using the out of the box
//! functionality provided by the penrose crate, working through [the book][5] before diving into
//! Penrose should be more than enough to get you started.
//!
//! On GitHub you can find up to date [examples][6] of how to set up and configure penrose as your
//! window manager, ranging from bare bones minimal to custom extensions and hooks.
//!
//!
//! # Getting started
//!
//! At it's simplest you will need to create a new binary crate to build your window manager and
//! add penrose as a project dependency:
//!
//! > $ cargo new --bin my_penrose_config
//!
//! As a bare minimum, you will need to the following in your **main.rs**:
//!   - keybindings (typically set up using the [gen_keybindings][7] macro)
//!   - A [XConn][8] instance to handle communication with the X server
//!   - A [Config][9] instance which contains the rest of your top level configuration for Penrose.
//!     Things like workspace names, layout functions and settings for gaps and borders.
//!
//! With that, you will be able to create a [WindowManager][10] and start running Penrose after
//! building and installing your binary. (It is also suggested that you set up a logging handler so
//! that debugging any issues with your config is easier. [simplelog][13] is a good choice if you
//! are unsure where to start with this.)
//!
//!
//! # Example
//!
//!```no_run
//! #[macro_use]
//! extern crate penrose;
//!
//! use penrose::{
//!     core::helpers::index_selectors,
//!     logging_error_handler,
//!     xcb::new_xcb_backed_window_manager,
//!     Backward, Config, Forward, Less, More, WindowManager
//! };
//!
//! fn main() -> penrose::Result<()> {
//!     let key_bindings = gen_keybindings! {
//!         "M-j" => run_internal!(cycle_client, Forward);
//!         "M-k" => run_internal!(cycle_client, Backward);
//!         "M-S-j" => run_internal!(drag_client, Forward);
//!         "M-S-k" => run_internal!(drag_client, Backward);
//!         "M-S-q" => run_internal!(kill_client);
//!         "M-Tab" => run_internal!(toggle_workspace);
//!         "M-grave" => run_internal!(cycle_layout, Forward);
//!         "M-S-grave" => run_internal!(cycle_layout, Backward);
//!         "M-A-Up" => run_internal!(update_max_main, More);
//!         "M-A-Down" => run_internal!(update_max_main, Less);
//!         "M-A-Right" => run_internal!(update_main_ratio, More);
//!         "M-A-Left" => run_internal!(update_main_ratio, Less);
//!         "M-semicolon" => run_external!("dmenu_run");
//!         "M-Return" => run_external!("alacritty");
//!         "M-A-Escape" => run_internal!(exit);
//!
//!         map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
//!             "M-{}" => focus_workspace (REF);
//!             "M-S-{}" => client_to_workspace (REF);
//!         };
//!     };
//!
//!     let mut wm = new_xcb_backed_window_manager(
//!         Config::default(),
//!         vec![],
//!         logging_error_handler()
//!     )?;
//!     wm.grab_keys_and_run(key_bindings, map!{})
//! }
//!```
//!
//! # Digging into the API
//!
//! To add more functionality and flexability, you can start to add things like [Hooks][11], a
//! [status bar][12] and custom actions for running as part of key bindings. You will want to read
//! the documentation of the `core` module which contains all of the core functionality of Penrose
//! as a window manager. After that, the `draw` module contains utilities for rendering things like
//! status bars and widgets, the `contrib` module has examples of simple hooks, extensions and key
//! binding actions and the `xcb` module contains the referencing trait implementations for
//! interacting with the X server via the [XCB][14] api.
//!
//! **NOTE**: in order to use the xcb implementation of penrose, you will need to install the C
//! libraries that are dependencies (namely xcb, Cairo and Pango).
//!
//! [1]: https://dwm.suckless.org/
//! [2]: https://xmonad.org/
//! [3]: http://www.qtile.org/
//! [4]: https://www.rust-lang.org/learn
//! [5]: https://doc.rust-lang.org/book/
//! [6]: https://github.com/sminez/penrose/tree/develop/examples
//! [7]: crate::gen_keybindings
//! [8]: crate::core::xconnection::XConn
//! [9]: crate::core::config::Config
//! [10]: crate::core::manager::WindowManager
//! [11]: crate::core::hooks
//! [12]: crate::draw::bar
//! [13]: https://crates.io/crates/simplelog
//! [14]: https://xcb.freedesktop.org/
//! [15]: https://www.rust-lang.org
#![warn(
    broken_intra_doc_links,
    clippy::all,
    missing_debug_implementations,
    future_incompatible,
    missing_docs,
    // missing_doc_code_examples,
    rust_2018_idioms,
)]
#![allow(clippy::too_many_arguments, clippy::clippy::borrowed_box)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg",
    issue_tracker_base_url = "https://github.com/sminez/penrose/issues/"
)]

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate log;

#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

#[macro_use]
pub mod core;

pub mod contrib;
pub mod draw;

#[cfg(feature = "xcb")]
pub mod xcb;

#[cfg(feature = "x11rb")]
pub mod x11rb;

#[doc(hidden)]
pub mod __example_helpers;

#[doc(hidden)]
pub use penrose_proc::validate_user_bindings;

// top level re-exports
#[doc(inline)]
pub use crate::core::{
    config::Config,
    data_types::Change::*,
    helpers::logging_error_handler,
    manager::WindowManager,
    ring::{Direction::*, InsertPoint, Selector},
    xconnection::Xid,
};

#[cfg(feature = "xcb")]
#[doc(inline)]
pub use crate::xcb::{new_xcb_backed_window_manager, XcbConnection};

/// Enum to store the various ways that operations can fail in Penrose
#[derive(thiserror::Error, Debug)]
pub enum PenroseError {
    /// Something went wrong using the [draw] module.
    ///
    /// See [DrawError][crate::draw::DrawError] for variants.
    #[error(transparent)]
    Draw(#[from] crate::draw::DrawError),

    /// Something was inconsistant when attempting to re-create a serialised [WindowManager]
    #[error("unable to rehydrate from serialized state: {0}")]
    HydrationState(String),

    /// Something was inconsistant when attempting to re-create a serialised [WindowManager]
    #[error("the following serialized client IDs were not known to the X server: {0:?}")]
    MissingClientIds(Vec<Xid>),

    /// A conversion to utf-8 failed
    #[error("UTF-8 error")]
    NonUtf8Prop(#[from] std::string::FromUtf8Error),

    #[doc(hidden)]
    #[error(transparent)]
    Infallible(#[from] std::convert::Infallible),

    /// An [IO Error][std::io::Error] was encountered
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Wm(Normal)Hints received from the X server were invalid
    #[error("Invalid window hints property: {0}")]
    InvalidHints(String),

    /// No elements match the given selector
    #[error("No elements match the given selector")]
    NoMatchingElement,

    /// Attempting to construct a penrose data type from an int failed.
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// A generic error type for use in user code when needing to construct
    /// a simple [PenroseError].
    #[error("Unhandled error: {0}")]
    Raw(String),

    /// An attempt to spawn an external process failed
    #[error("unable to get stdout handle for child process: {0}")]
    SpawnProc(String),

    /// Parsing an [Atom][core::xconnection::Atom] from a str failed.
    ///
    /// This happens when the atom name being requested is not a known atom.
    #[error(transparent)]
    Strum(#[from] strum::ParseError),

    /// An attempt was made to reference a client that is not known to penrose
    #[error("{0} is not a known client")]
    UnknownClient(Xid),

    /// A user specified key binding contained an invalid modifier key
    #[error("Unknown modifier key: {0}")]
    UnknownModifier(String),

    /// Something went wrong using the [xcb] module.
    ///
    /// See [XcbError][crate::xcb::XcbError] for variants.
    #[cfg(feature = "xcb")]
    #[error(transparent)]
    Xcb(#[from] crate::xcb::XcbError),

    /// Something went wrong using the [x11rb] module.
    ///
    /// See [X11rbError][crate::x11rb::X11rbError] for variants.
    #[cfg(feature = "x11rb")]
    #[error(transparent)]
    X11rb(#[from] crate::x11rb::X11rbError),

    /// Something went wrong when communicating with the X server
    #[error(transparent)]
    X(#[from] crate::core::xconnection::XError),
}

/// Top level penrose Result type
pub type Result<T> = std::result::Result<T, PenroseError>;

/// A function that can be registered to handle errors that occur during [WindowManager] operation
pub type ErrorHandler = Box<dyn FnMut(PenroseError)>;
