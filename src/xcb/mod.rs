//! Helpers and utilities for using XCB as a back end for penrose
//!
//! This is the reference implementation of the core Penrose traits that are required to back the
//! [WindowManager][1] when talking to the X server. The code in this module is build on top of the
//! [xcb][2], [pango][3] and [cairo][4] C libraries and strives to be as simple as possible (as
//! opposed to optimising for performance).
//!
//! # Available features
//! - `xcb_draw`: adds `pango` and `cairo` as build dependencies and provides implementations of
//!   the [Draw][6] and [DrawContext][7] traits.
//! - `keysyms`: enable [KeyPressDraw][8] functionality for [XcbDraw][9]
//!
//! # C level documentation
//!
//! Docs for the underlying `xcb` C library can be found [here][10].
//!
//! [1]: crate::core::manager::WindowManager
//! [2]: https://xcb.freedesktop.org/
//! [3]: https://www.pango.org/
//! [4]: https://www.cairographics.org/
//! [5]: penrose_keysyms::XKeySym
//! [6]: crate::draw::Draw
//! [7]: crate::draw::DrawContext
//! [8]: crate::draw::KeyPressDraw
//! [9]: crate::xcb::XcbDraw
//! [10]: https://www.mankier.com/package/libxcb-devel
use crate::{
    core::{
        config::Config,
        data_types::WinId,
        hooks::{Hook, Hooks},
        manager::WindowManager,
    },
    ErrorHandler,
};

#[cfg(feature = "xcb_draw")]
use crate::draw::{dwm_bar, Color, StatusBar, TextStyle};

pub mod api;
#[doc(hidden)]
pub mod conversions;
#[cfg(feature = "xcb_draw")]
pub mod draw;
pub mod helpers;
pub mod xconn;

#[doc(inline)]
pub use api::Api;
#[doc(inline)]
#[cfg(feature = "xcb_draw")]
pub use draw::{XcbDraw, XcbDrawContext};
#[doc(inline)]
pub use xconn::XcbConnection;

/// A generic event type returned by the xcb library
pub type XcbGenericEvent = xcb::Event<xcb::ffi::base::xcb_generic_event_t>;

/// Result type for fallible methods using XCB
pub type Result<T> = std::result::Result<T, XcbError>;

/// Helper type for when you are defining your [Hook] vector in your main.rs when using
/// the default XCB impls
pub type XcbHooks = Hooks<XcbConnection>;

/// Construct a penrose [WindowManager] backed by the default [xcb][crate::xcb] backend.
pub fn new_xcb_backed_window_manager(
    config: Config,
    hooks: Vec<Box<dyn Hook<XcbConnection>>>,
    error_handler: ErrorHandler,
) -> crate::Result<WindowManager<XcbConnection>> {
    let conn = XcbConnection::new()?;
    let mut wm = WindowManager::new(config, conn, hooks, error_handler);
    wm.init()?;

    Ok(wm)
}

/// Construct a new [StatusBar] using the default [dwm_bar] configuration, backed by [XcbDraw]
pub fn new_xcb_backed_status_bar(
    height: usize,
    style: &TextStyle,
    highlight: impl Into<Color>,
    empty_ws: impl Into<Color>,
    workspaces: Vec<impl Into<String>>,
) -> crate::draw::Result<StatusBar<XcbDrawContext, XcbDraw, XcbConnection>> {
    dwm_bar(
        XcbDraw::new()?,
        height,
        style,
        highlight,
        empty_ws,
        workspaces,
    )
}

/// Enum to store the various ways that operations can fail inside of the
/// XCB implementations of penrose traits.
#[derive(thiserror::Error, Debug)]
pub enum XcbError {
    /// Unable to establish a connection to the X server
    #[error("Unable to connect to the X server via XCB")]
    Connection(#[from] ::xcb::ConnError),

    /// A xcb query failed to return a value
    #[error("Xcb query returned None: {0}")]
    EmptyResponse(String),

    /// An [IO Error][std::io::Error] was encountered
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// A requested client property was empty
    #[error("'{0}' prop is not set for client {1}")]
    MissingProp(String, WinId),

    /// Property data returned for the target window was in an invalid format
    #[error("invalid property data: {0}")]
    InvalidPropertyData(String),

    /// No screens were found
    #[error("Unable to fetch setup roots from XCB")]
    NoScreens,

    /// A string property on a window was invalid utf8
    #[error("Requested property was not valid UTF8")]
    NonUtf8Prop(#[from] std::string::FromUtf8Error),

    /// An attempt to determine a certain property of the running system failed
    #[error("Unable to determine required value: {0}")]
    QueryFailed(&'static str),

    /// A query via the randr API was unsuccessful
    #[error("randr query failed: {0}")]
    Randr(String),

    /// A generic error type for use in user code when needing to construct
    /// a simple [XcbError].
    #[error("Unhandled error: {0}")]
    Raw(String),

    /// Parsing a strum generated enum from a str failed.
    #[error(transparent)]
    Strum(#[from] strum::ParseError),

    /// Screen data for an unknown screen was requested
    #[error("The requested screen index was out of bounds: {0} > {1}")]
    UnknownScreen(usize, usize),

    /// Wrapper around low level X11 errors
    #[error("X11 error: error seq={0}, code={1}, xid={2}, request: {3}:{4}")]
    X11Error(u16, u8, u32, u8, u16),

    /// Wrapper around low level XCB C API errors
    #[error("Error making xcb query: {0}")]
    XcbGeneric(#[from] ::xcb::Error<::xcb::ffi::base::xcb_generic_error_t>),

    /// Error in using the pango API
    #[cfg(feature = "xcb_draw")]
    #[error("Error calling Pango API: {0}")]
    Pango(String),

    /// An attempt was made to fetch a surface for a client before creating it
    #[cfg(feature = "xcb_draw")]
    #[error("no cairo surface for {0}")]
    UnintialisedSurface(WinId),

    /// A user specified mouse binding contained an invalid button
    #[error("Unknown mouse button: {0}")]
    UnknownMouseButton(u8),
}
