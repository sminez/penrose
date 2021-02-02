//! Data types for working with X events
use crate::core::{
    bindings::{KeyCode, MouseEvent},
    data_types::{Point, Region},
    xconnection::Xid,
};

/// Wrapper around the low level X event types that correspond to request / response data when
/// communicating with the X server itself.
///
/// The variant names and data have developed with the reference xcb implementation in mind but
/// should be applicable for all back ends.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub enum XEvent {
    /// The mouse has moved or a mouse button has been pressed
    MouseEvent(MouseEvent),

    /// A grabbed key combination has been entered by the user
    KeyPress(KeyCode),

    /// A client window is requesting to be positioned and rendered on the screen
    MapRequest {
        /// The ID of the window that wants to be mapped
        id: Xid,
        /// Whether or not the WindowManager should handle this window.
        ignore: bool,
    },

    /// The mouse pointer has entered a new client window
    Enter {
        /// The ID of the window that was entered
        id: Xid,
        /// Absolute coordinate of the event
        rpt: Point,
        /// Coordinate of the event relative to top-left of the window itself
        wpt: Point,
    },

    /// The mouse pointer has left the current client window
    Leave {
        /// The ID of the window that was left
        id: Xid,
        /// Absolute coordinate of the event
        rpt: Point,
        /// Coordinate of the event relative to top-left of the window itself
        wpt: Point,
    },

    /// A client window has been closed
    Destroy {
        /// The ID of the window being destroyed
        id: Xid,
    },

    /// Focus has moved to a different screen
    ScreenChange,

    /// A randr action has occured (new outputs, resolution change etc)
    RandrNotify,

    /// Client config has changed in some way
    ConfigureNotify {
        /// The ID of the window that had a property changed
        id: Xid,
        /// The new window size
        r: Region,
        /// Is this window the root window?
        is_root: bool,
    },

    /// A client is requesting to be repositioned
    ConfigureRequest {
        /// The ID of the window that had a property changed
        id: Xid,
        /// The new window size
        r: Region,
        /// Is this window the root window?
        is_root: bool,
    },

    /// A part or all of a client has become visible
    Expose {
        /// The ID of the window that has become exposed
        id: Xid,
        /// The current size and position of the window
        r: Region,
        /// How many following expose events are pending
        count: usize,
    },

    /// A client property has changed in some way
    PropertyNotify {
        /// The ID of the window that had a property changed
        id: Xid,
        /// The property that changed
        atom: String,
        /// Is this window the root window?
        is_root: bool,
    },

    /// A message has been sent to a particular client
    ClientMessage {
        /// The ID of the window that sent the message
        id: Xid,
        /// The data type being set
        dtype: String,
        /// The data itself
        data: Vec<usize>,
    },
}
