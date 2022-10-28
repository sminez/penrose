//! Data types for working with X events
use crate::{
    core::bindings::{KeyCode, MouseEvent},
    pure::geometry::{Point, Rect},
    x::{Atom, XConn},
    Result, Xid,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

/// Wrapper around the low level X event types that correspond to request / response data when
/// communicating with the X server itself.
///
/// The variant names and data have developed with the reference xcb implementation in mind but
/// should be applicable for all back ends.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum XEvent {
    /// A message has been sent to a particular client
    ClientMessage(ClientMessage),
    /// Client config has changed in some way
    ConfigureNotify(ConfigureEvent),
    /// A client is requesting to be repositioned
    ConfigureRequest(ConfigureEvent),
    /// The mouse pointer has entered a new client window
    Enter(PointerChange),
    /// A part or all of a client has become visible
    Expose(ExposeEvent),
    /// A client should have focus
    FocusIn(Xid),
    /// A client window has been closed
    Destroy(Xid),
    /// A grabbed key combination has been entered by the user
    KeyPress(KeyCode),
    /// The mouse pointer has left the current client window
    Leave(PointerChange),
    /// Keybindings have changed
    MappingNotify,
    /// A client window is requesting to be positioned and rendered on the screen.
    MapRequest(Xid),
    /// The mouse has moved or a mouse button has been pressed
    MouseEvent(MouseEvent),
    /// A client property has changed in some way
    PropertyNotify(PropertyEvent),
    /// A randr action has occured (new outputs, resolution change etc)
    RandrNotify,
    /// Focus has moved to a different screen
    ScreenChange,
    /// A client is being unmapped
    UnmapNotify(Xid),
}

impl std::fmt::Display for XEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use XEvent::*;

        match self {
            ClientMessage(_) => write!(f, "ClientMessage"),
            ConfigureNotify(_) => write!(f, "ConfigureNotify"),
            ConfigureRequest(_) => write!(f, "ConfigureRequest"),
            Enter(_) => write!(f, "Enter"),
            Expose(_) => write!(f, "Expose"),
            FocusIn(_) => write!(f, "FocusIn"),
            Destroy(_) => write!(f, "Destroy"),
            KeyPress(_) => write!(f, "KeyPress"),
            Leave(_) => write!(f, "Leave"),
            MappingNotify => write!(f, "MappingNotify"),
            MapRequest(_) => write!(f, "MapRequest"),
            MouseEvent(_) => write!(f, "MouseEvent"),
            PropertyNotify(_) => write!(f, "PropertyNotify"),
            RandrNotify => write!(f, "RandrNotify"),
            ScreenChange => write!(f, "ScreenChange"),
            UnmapNotify(_) => write!(f, "UnmapNotify"),
        }
    }
}

/// Known common client message formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientMessageKind {
    /// Inform a client that it is being closed
    DeleteWindow(Xid),
    /// Request that a client take input focus
    TakeFocus(Xid),
    /// Take ownership of the systray
    ///
    /// Args are the id of the root window and id of the window being used as a systray
    TakeSystrayOwnership(Xid, Xid),
    /// Inform an embedded window that it has gained focus
    XEmbedFocusIn(Xid, Xid),
    /// Inform an embedded window that it has been blocked by a modal dialog
    XEmbedModalityOn(Xid, Xid),
    /// Inform a window that it is being embedded
    XEmbedNotify(Xid, Xid),
    /// Inform an embedded window that it is now active
    XEmbedWindowActivate(Xid, Xid),
}

impl ClientMessageKind {
    /// Build a default [ClientMessage] compatible with X11 / XCB formats.
    ///
    /// Most impls of `X*` traits should be able to use the default data generated by this method,
    /// but if you need to send something else, you can always construct the `ClientMessage`
    /// explicitly.
    pub fn as_message<X>(&self, q: &X) -> Result<ClientMessage>
    where
        X: XConn,
    {
        let proto_msg = |id: Xid, atom: Atom| {
            let proto = Atom::WmProtocols.as_ref();
            let data = &[*q.intern_atom(atom.as_ref())?, 0, 0, 0, 0];
            let mask = ClientEventMask::NoEventMask;

            Ok(ClientMessage::new(id, mask, proto, data.into()))
        };

        // https://specifications.freedesktop.org/xembed-spec/xembed-spec-latest.html
        let xembed_version = 0;
        let notify = 0;
        let activate = 1;
        let focus_in = 4;
        let modality_on = 10;

        let xembed_msg = |id: Xid, embedder: Xid, kind: u32| {
            let atom = Atom::XEmbed.as_ref();
            let data = &[0, kind, 0, *embedder, xembed_version];
            let mask = ClientEventMask::SubstructureNotify;

            Ok(ClientMessage::new(id, mask, atom, data.into()))
        };

        match self {
            ClientMessageKind::DeleteWindow(id) => proto_msg(*id, Atom::WmDeleteWindow),
            ClientMessageKind::TakeFocus(id) => proto_msg(*id, Atom::WmTakeFocus),

            ClientMessageKind::TakeSystrayOwnership(root_id, systray_id) => {
                let atom = Atom::Manager.as_ref();
                let systray = q.intern_atom(Atom::NetSystemTrayS0.as_ref())?;
                let data = &[0, *systray, **systray_id, 0, 0];
                let mask = ClientEventMask::SubstructureNotify;

                Ok(ClientMessage::new(*root_id, mask, atom, data.into()))
            }

            ClientMessageKind::XEmbedFocusIn(id, other) => xembed_msg(*id, *other, focus_in),
            ClientMessageKind::XEmbedModalityOn(id, other) => xembed_msg(*id, *other, modality_on),
            ClientMessageKind::XEmbedNotify(id, other) => xembed_msg(*id, *other, notify),
            ClientMessageKind::XEmbedWindowActivate(id, other) => xembed_msg(*id, *other, activate),
        }
    }
}

/// Event masks used when sending client events
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientEventMask {
    /// Substructure Notify
    SubstructureNotify,
    /// Structure Notify
    StructureNotify,
    /// No Mask: all clients should accept
    NoEventMask,
}

/// The raw data contained in a [`ClientMessage`]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClientMessageData {
    /// Slice of u8
    U8([u8; 20]),
    /// Slice of u16
    U16([u16; 10]),
    /// Slice of u32
    U32([u32; 5]),
}

macro_rules! cast_slice {
    ($s:expr, $t:ty) => {
        $s.iter().map(|&v| v as $t).collect::<Vec<$t>>()
    };
}

impl ClientMessageData {
    /// Convert this client message into a single data format
    ///
    /// The number of raw values will be maintained but this allows you to have a consistant
    /// interface without needing to match on the variant or cast all the time.
    pub fn as_usize(&self) -> Vec<usize> {
        match self {
            Self::U8(data) => cast_slice!(data, usize),
            Self::U16(data) => cast_slice!(data, usize),
            Self::U32(data) => cast_slice!(data, usize),
        }
    }
}

macro_rules! __impl_client_message_data(
    { $t:ty; $count:expr, $variant:expr, $method:ident } => {
        impl ClientMessageData {
            /// Convert this client message into a single data format
            ///
            /// The number of raw values will be maintained but this allows you to have a consistant
            /// interface without needing to match on the variant or cast all the time.
            pub fn $method(&self) -> Vec<$t> {
                match self {
                    Self::U8(data) => cast_slice!(data, $t),
                    Self::U16(data) => cast_slice!(data, $t),
                    Self::U32(data) => cast_slice!(data, $t),
                }
            }
        }
        impl From<[$t; $count]> for ClientMessageData {
            fn from(data: [$t; $count]) -> Self {
                $variant(data)
            }
        }
        impl From<&[$t; $count]> for ClientMessageData {
            fn from(data: &[$t; $count]) -> Self {
                $variant(*data)
            }
        }
        impl TryFrom<&[$t]> for ClientMessageData {
            type Error = std::array::TryFromSliceError;

            fn try_from(data: &[$t]) -> std::result::Result<Self, Self::Error> {
                Ok($variant(<[$t; $count]>::try_from(data)?))
            }
        }
    }
);

__impl_client_message_data!(u8; 20, ClientMessageData::U8, as_u8);
__impl_client_message_data!(u16; 10, ClientMessageData::U16, as_u16);
__impl_client_message_data!(u32; 5, ClientMessageData::U32, as_u32);

/// A client message that needs to be parsed and handled based on its type
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientMessage {
    /// The ID of the window that sent the message
    pub id: Xid,
    /// The mask to use when sending the event
    pub mask: ClientEventMask,
    /// The data type being set
    pub dtype: String,
    /// The raw data being sent in this message
    pub data: ClientMessageData,
}

impl ClientMessage {
    /// Try to build a new ClientMessage. Fails if the data is invalid
    pub fn new(
        id: Xid,
        mask: ClientEventMask,
        dtype: impl Into<String>,
        data: ClientMessageData,
    ) -> Self {
        Self {
            id,
            mask,
            dtype: dtype.into(),
            data,
        }
    }
}

/// A configure request or notification when a client changes position or size
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConfigureEvent {
    /// The ID of the window that had a property changed
    pub id: Xid,
    /// The new window size
    pub r: Rect,
    /// Is this window the root window?
    pub is_root: bool,
}

/// A notification that a window has become visible
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExposeEvent {
    /// The ID of the window that has become exposed
    pub id: Xid,
    /// The current size and position of the window
    pub r: Rect,
    /// How many following expose events are pending
    pub count: usize,
}

/// A notification that the mouse pointer has entered or left a window
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointerChange {
    /// The ID of the window that was entered
    pub id: Xid,
    /// Absolute coordinate of the event
    pub abs: Point,
    /// Coordinate of the event relative to top-left of the window itself
    pub relative: Point,
    /// Whether or not the event window is on the same screen as the root window
    pub same_screen: bool,
}

/// A property change on a known client
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropertyEvent {
    /// The ID of the window that had a property changed
    pub id: Xid,
    /// The property that changed
    pub atom: String,
    /// Is this window the root window?
    pub is_root: bool,
}
