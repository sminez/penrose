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
//! [7]: crate::draw::KeyPressDraw
//! [7]: crate::xcb::XcbDraw
//! [10]: https://www.mankier.com/package/libxcb-devel
use crate::{
    core::{
        bindings::{KeyCode, MouseState},
        config::Config,
        data_types::{Point, PropVal, Region, WinAttr, WinConfig, WinId, WinType},
        hooks::{Hook, Hooks},
        manager::WindowManager,
        screen::Screen,
        xconnection::{Atom, XEvent},
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
    #[error("'{}' prop is not set for client {1}", .0.as_ref())]
    MissingProp(Atom, WinId),

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

    /// Wrapper around low level XCB C API errors
    #[error("Error making xcb query")]
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

/**
 * An abstraction layer for talking to the X server using the XCB api.
 *
 * This has been written to be a reasonably close mapping to the underlying
 * C API, but provides several quality of life changes that make consuming
 * the API nicer to work with in Penrose code.
 */
pub trait XcbApi {
    /// Hydrate this XcbApi to restore internal state following serde deserialization
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()>;

    /**
     * Intern an atom by name, returning the corresponding id.
     *
     * Can fail if the atom name is not a known X atom or if there
     * is an issue with communicating with the X server. For known
     * atoms that are included in the [Atom] enum,
     * the [XcbApi::known_atom] method should be used instead.
     */
    fn atom(&self, name: &str) -> Result<u32>;

    /**
     * Fetch the id value of a known [Atom] variant.
     *
     * This operation is expected to always succeed as known atoms should
     * either be interned on init of the implementing struct or statically
     * assigned a value in the implementation.
     */
    fn known_atom(&self, atom: Atom) -> u32;

    /// Delete a known property from a window
    fn delete_prop(&self, id: WinId, prop: Atom);
    /// Fetch an [Atom] property for a given window
    fn get_atom_prop(&self, id: WinId, atom: Atom) -> Result<u32>;
    /// Fetch an String property for a given window
    fn get_str_prop(&self, id: WinId, name: &str) -> Result<String>;
    /**
     * Replace a property value on a window.
     *
     * See the documentation for the C level XCB API for the correct property
     * type for each prop.
     */
    fn replace_prop(&self, id: WinId, prop: Atom, val: PropVal<'_>);

    /// Create a new client window
    fn create_window(&self, ty: WinType, r: Region, managed: bool) -> Result<WinId>;
    /// Apply a set of config options to a window
    fn configure_window(&self, id: WinId, conf: &[WinConfig]);
    /// The list of currently active clients known to the X server
    fn current_clients(&self) -> Result<Vec<WinId>>;
    /// Destroy the X server state for a given window
    fn destroy_window(&self, id: WinId);
    /// The client that the X server currently considers to be focused
    fn focused_client(&self) -> Result<WinId>;
    /// Send a [XEvent::MapRequest] for the target window
    fn map_window(&self, id: WinId);
    /// Mark the given window as currently having focus in the X server state
    fn mark_focused_window(&self, id: WinId);
    /// Send an event to a client
    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()>;
    /// Set attributes on the target window
    fn set_window_attributes(&self, id: WinId, attrs: &[WinAttr]);
    /// Unmap the target window
    fn unmap_window(&self, id: WinId);
    /// Find the current size and position of the target window
    fn window_geometry(&self, id: WinId) -> Result<Region>;

    /// Query the randr API for current outputs and return the details as penrose
    /// [Screen] structs.
    fn current_screens(&self) -> Result<Vec<Screen>>;
    /// Query the randr API for current outputs and return the size of each screen
    fn screen_sizes(&self) -> Result<Vec<Region>>;

    /// The current (x, y) position of the cursor relative to the root window
    fn cursor_position(&self) -> Point;
    /// Register intercepts for each given [KeyCode]
    fn grab_keys(&self, keys: &[&KeyCode]);
    /// Register intercepts for each given [MouseState]
    fn grab_mouse_buttons(&self, states: &[&MouseState]);
    /// Drop all active intercepts for key combinations
    fn ungrab_keys(&self);
    /// Drop all active intercepts for mouse states
    fn ungrab_mouse_buttons(&self);

    /// Flush pending actions to the X event loop
    fn flush(&self) -> bool;
    /// The current root window ID
    fn root(&self) -> WinId;
    /// Set a pre-defined notify mask for randr events to subscribe to
    fn set_randr_notify_mask(&self) -> Result<()>;
    /**
     * Block until the next event from the X event loop is ready then return it.
     *
     * This method should handle all of the mapping of xcb events to penrose
     * [XEvent] instances, returning an Error when the event channel from the
     * X server is closed.
     */
    fn wait_for_event(&self) -> Result<XEvent>;
    /**
     * Return the next event from the X event loop if there is one.
     *
     * This method should handle all of the mapping of xcb events to penrose
     * [XEvent] instances, returning None if there is no pending event and an error
     * if the connection to the X server is closed.
     */
    fn poll_for_event(&self) -> Result<Option<XEvent>>;
    /// Move the cursor to the given (x, y) position inside the specified window.
    fn warp_cursor(&self, id: WinId, x: usize, y: usize);
}
