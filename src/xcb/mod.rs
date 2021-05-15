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
        hooks::{Hook, Hooks},
        manager::WindowManager,
        xconnection::Xid,
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
    MissingProp(String, Xid),

    /// Property data returned for the target window was in an invalid format
    #[error("invalid client message data: format={0}")]
    InvalidClientMessage(u8),

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
    #[error("Error making xcb query: {0:?}")]
    XcbKnown(XErrorCode),

    /// Error in using the pango API
    #[cfg(feature = "xcb_draw")]
    #[error("Error calling Pango API: {0}")]
    Pango(String),

    /// An attempt was made to fetch a surface for a client before creating it
    #[cfg(feature = "xcb_draw")]
    #[error("no cairo surface for {0}")]
    UnintialisedSurface(Xid),

    /// A user specified mouse binding contained an invalid button
    #[error("Unknown mouse button: {0}")]
    UnknownMouseButton(u8),

    /// Wrapper around low level XCB C API errors
    #[error("Unknown error making xcb query: error_code={0} response_type={1}")]
    XcbUnknown(u8, u8),
}

fn from_error_code(code: u8, response_type: u8) -> XcbError {
    match code {
        1..=11 => XcbError::XcbKnown(unsafe { std::mem::transmute(code) }),
        _ => XcbError::XcbUnknown(code, response_type),
    }
}

impl From<::xcb::GenericError> for XcbError {
    fn from(raw: ::xcb::GenericError) -> Self {
        from_error_code(raw.error_code(), raw.response_type())
    }
}

impl From<&::xcb::GenericError> for XcbError {
    fn from(raw: &::xcb::GenericError) -> Self {
        from_error_code(raw.error_code(), raw.response_type())
    }
}

/// Base X11 error codes taken from /usr/include/X11/X.h (line 347)
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Ord, PartialOrd)]
pub enum XErrorCode {
    /// bad request code
    BadRequest = 1,
    /// int parameter out of range
    BadValue = 2,
    /// parameter not a Window
    BadWindow = 3,
    /// parameter not a Pixmap
    BadPixmap = 4,
    /// parameter not an Atom
    BadAtom = 5,
    /// parameter not a Cursor
    BadCursor = 6,
    /// parameter not a Font
    BadFont = 7,
    /// parameter mismatch
    BadMatch = 8,
    /// parameter not a Pixmap or Window
    BadDrawable = 9,
    /// depending on context:
    ///   - key/button already grabbed
    ///   - attempt to free an illegal cmap entry
    ///   - attempt to store into a read-only color map entry.
    ///   - attempt to modify the access control list from other than the local host.
    BadAccess = 10,
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xatom_querier {
    { $struct:ident } => {
        impl $crate::core::xconnection::XAtomQuerier for $struct {
            fn atom_name(&self, atom: Xid) -> $crate::core::xconnection::Result<String> {
                Ok(self.api.atom_name(atom)?)
            }

            fn atom_id(&self, name: &str) -> $crate::core::xconnection::Result<Xid> {
                Ok(self.api.atom(name)?)
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xstate {
    { $struct:ident } => {
        impl $crate::core::xconnection::XState for $struct {
            fn root(&self) -> Xid {
                self.api.root()
            }

            fn current_screens(&self) -> $crate::core::xconnection::Result<Vec<Screen>> {
                Ok(self.api.current_screens()?)
            }

            fn cursor_position(&self) -> $crate::core::xconnection::Result<Point> {
                Ok(self.api.cursor_position()?)
            }

            fn warp_cursor(&self, win_id: Option<Xid>, screen: &Screen) -> $crate::core::xconnection::Result<()> {
                let (x, y, id) = match win_id {
                    Some(id) => {
                        let (_, _, w, h) = self.client_geometry(id)?.values();
                        ((w / 2), (h / 2), id)
                    }
                    None => {
                        let (x, y, w, h) = screen.region(true).values();
                        ((x + w / 2), (y + h / 2), self.api.root())
                    }
                };

                Ok(self.api.warp_cursor(id, x as usize, y as usize)?)
            }

            fn client_geometry(&self, id: Xid) -> $crate::core::xconnection::Result<Region> {
                Ok(self.api.client_geometry(id)?)
            }

            fn active_clients(&self) -> $crate::core::xconnection::Result<Vec<Xid>> {
                Ok(self.api.current_clients()?)
            }

            fn focused_client(&self) -> $crate::core::xconnection::Result<Xid> {
                Ok(self.api.focused_client()?)
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xeventhandler {
    { $struct:ident } => {
        impl $crate::core::xconnection::XEventHandler for $struct {
            fn flush(&self) -> bool {
                self.api.flush()
            }

            fn wait_for_event(&self) -> $crate::core::xconnection::Result<XEvent> {
                Ok(self.api.wait_for_event()?)
            }

            fn send_client_event(&self, msg: ClientMessage) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.send_client_event(msg)?)
            }

            fn build_client_event(&self, kind: ClientMessageKind) -> $crate::core::xconnection::Result<ClientMessage> {
                self.api.build_client_event(kind)
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xclienthandler {
    { $struct:ident } => {
        impl $crate::core::xconnection::XClientHandler for $struct {
            fn map_client(&self, id: Xid) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.map_client(id)?)
            }

            fn unmap_client(&self, id: Xid) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.unmap_client(id)?)
            }

            fn focus_client(&self, id: Xid) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.focus_client(id)?)
            }

            fn destroy_client(&self, id: Xid) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.destroy_client(id)?)
            }

            fn kill_client(&self, id: Xid) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.kill_client(id)?)
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xclientproperties {
    { $struct:ident } => {
        impl $crate::core::xconnection::XClientProperties for $struct {
            fn get_prop(&self, id: Xid, name: &str) -> $crate::core::xconnection::Result<Prop> {
                match self.api.get_prop(id, name) {
                    Err(XcbError::XcbKnown($crate::xcb::XErrorCode::BadAtom)) => {
                        Err($crate::core::xconnection::XError::MissingProperty(name.into(), id))
                    },
                    other => Ok(other?),
                }
            }

            fn list_props(&self, id: Xid) -> $crate::core::xconnection::Result<Vec<String>> {
                Ok(self.api.list_props(id)?)
            }

            fn delete_prop(&self, id: Xid, name: &str) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.delete_prop(id, name)?)
            }

            fn change_prop(&self, id: Xid, prop: &str, val: Prop) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.change_prop(id, prop, val)?)
            }

            fn set_client_state(&self, id: Xid, state: WindowState) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.set_client_state(id, state)?)
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xclientconfig {
    { $struct:ident } => {
        impl $crate::core::xconnection::XClientConfig for $struct {
            fn configure_client(&self, id: Xid, data: &[ClientConfig]) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.configure_client(id, data)?)
            }

            fn set_client_attributes(&self, id: Xid, data: &[ClientAttr]) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.set_client_attributes(id, data)?)
            }

            fn get_window_attributes(&self, id: Xid) -> $crate::core::xconnection::Result<$crate::core::xconnection::WindowAttributes> {
                Ok(self.api.get_window_attributes(id)?)
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! __xcb_impl_xkeyboardhandler {
    { $struct:ident } => {
        impl XKeyboardHandler for $struct {
            fn grab_keyboard(&self) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.grab_keyboard()?)
            }

            fn ungrab_keyboard(&self) -> $crate::core::xconnection::Result<()> {
                Ok(self.api.ungrab_keyboard()?)
            }

            fn next_keypress(&self) -> $crate::core::xconnection::Result<Option<KeyPressParseAttempt>> {
                Ok(self.api.next_keypress()?)
            }

            fn next_keypress_blocking(&self) -> $crate::core::xconnection::Result<KeyPressParseAttempt> {
                Ok(self.api.next_keypress_blocking()?)
            }
        }
    }
}
