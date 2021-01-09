//! A tiling window manager library in the style of Xmonad
//!
//!
//!```no_run
//! #[macro_use]
//! extern crate penrose;
//!
//! use penrose::{
//!     core::{
//!         config::Config, helpers::index_selectors, manager::WindowManager,
//!     },
//!     xcb::new_xcb_backed_window_manager,
//!     Backward, Forward, Less, More, Result, logging_error_handler, EventSource
//! };
//!
//! fn main() -> Result<()> {
//!     let config = Config::default();
//!     let hooks = vec![];
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
//!         "M-A-Escape" => run_internal!(exit);
//!         "M-semicolon" => run_external!("dmenu_run");
//!         "M-Return" => run_external!("st");
//!
//!         refmap [ config.ws_range() ] in {
//!             "M-{}" => focus_workspace [ index_selectors(config.workspaces().len()) ];
//!             "M-S-{}" => client_to_workspace [ index_selectors(config.workspaces().len()) ];
//!         };
//!     };
//!
//!     let mut wm = new_xcb_backed_window_manager(
//!         config,
//!         hooks,
//!         logging_error_handler(),
//!         EventSource::NonBlocking
//!     )?;
//!     wm.grab_keys_and_run(key_bindings, map!{})
//! }
//!```
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

// top level re-exports
#[doc(inline)]
pub use crate::core::{
    config::Config,
    data_types::{Change::*, EventSource, WinId},
    helpers::logging_error_handler,
    manager::WindowManager,
    ring::{Direction::*, InsertPoint, Selector},
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
    MissingClientIds(Vec<WinId>),

    /// An [IO Error][std::io::Error] was encountered
    #[error(transparent)]
    Io(#[from] std::io::Error),

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

    /// A user specified key binding contained an invalid modifier key
    #[error("Unknown modifier key: {0}")]
    UnknownModifier(String),

    /// Something went wrong using the [xcb] module.
    ///
    /// See [XcbError][crate::xcb::XcbError] for variants.
    #[cfg(feature = "xcb")]
    #[error(transparent)]
    Xcb(#[from] crate::xcb::XcbError),
}

/// Top level penrose Result type
pub type Result<T> = std::result::Result<T, PenroseError>;

/// A function that can be registered to handle errors that occur during [WindowManager] operation
pub type ErrorHandler = Box<dyn FnMut(PenroseError)>;
