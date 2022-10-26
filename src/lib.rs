//! # Penrose: a library for building your very own tiling window manager
#[cfg(feature = "x11rb-xcb")]
use ::x11rb::{
    errors::{ConnectError, ConnectionError, ReplyError, ReplyOrIdError},
    x11_utils::X11Error,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::any::TypeId;

pub mod builtin;
pub mod core;
pub mod extensions;
pub mod macros;
pub mod pure;
pub mod util;
pub mod x;
#[cfg(feature = "x11rb-xcb")]
pub mod x11rb;

pub use crate::core::Xid;
pub use pure::{
    geometry::{Point, Rect},
    Position, Screen, Stack, StackSet, Workspace,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A custom error message from user code or extensions
    #[error("{0}")]
    Custom(String),

    #[error("Only {n_ws} workspaces were provided but at least {n_screens} are required")]
    InsufficientWorkspaces { n_ws: usize, n_screens: usize },

    #[error("invalid client message data: format={format}")]
    InvalidClientMessage { format: u8 },

    #[error("Invalid Hex color code: '{hex_code}'")]
    InvalidHexColor { hex_code: String },

    #[error("Invalid window hints message: {reason}")]
    InvalidHints { reason: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    #[error("{ty} property '{prop}' for {id} contained invalid data")]
    InvalidPropertyData { id: Xid, ty: String, prop: String },

    #[error("There are no screens available")]
    NoScreens,

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("Error initialising randr: {0}")]
    Randr(String),

    #[error("The given client is not in this State")]
    UnknownClient,

    #[error("'{name}' is not a known key name")]
    UnknownKeyName { name: String },

    #[error("'{name}' is not a known modifier key")]
    UnknownModifier { name: String },

    #[error("{button} is not a supported mouse button")]
    UnknownMouseButton { button: u8 },

    #[error("{type_id:?} was requested as a state extension but not found")]
    UnknownStateExtension { type_id: TypeId },

    // TODO: These backend specific errors should be abstracted out to a
    //       set of common error variants that they can be mapped to without
    //       needing to extend the enum conditionally when flags are enabled
    //
    #[cfg(feature = "x11rb-xcb")]
    #[error(transparent)]
    X11rbConnect(#[from] ConnectError),

    #[cfg(feature = "x11rb-xcb")]
    #[error(transparent)]
    X11rbConnection(#[from] ConnectionError),

    #[cfg(feature = "x11rb-xcb")]
    #[error(transparent)]
    X11rbReplyError(#[from] ReplyError),

    #[cfg(feature = "x11rb-xcb")]
    #[error(transparent)]
    X11rbReplyOrIdError(#[from] ReplyOrIdError),

    #[cfg(feature = "x11rb-xcb")]
    #[error("X11 error: {0:?}")]
    X11rbX11Error(X11Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq)]
/// A simple RGBA based color
pub struct Color {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

// helper for methods in Color
macro_rules! _f2u { { $f:expr, $s:expr } => { (($f * 255.0) as u32) << $s } }

impl Color {
    /// Create a new Color from a hex encoded u32: 0xRRGGBB or 0xRRGGBBAA
    pub fn new_from_hex(hex: u32) -> Self {
        let floats: Vec<f64> = hex
            .to_be_bytes()
            .iter()
            .map(|n| *n as f64 / 255.0)
            .collect();

        let (r, g, b, a) = (floats[0], floats[1], floats[2], floats[3]);
        Self { r, g, b, a }
    }

    /// The RGB information of this color as 0.0-1.0 range floats representing
    /// proportions of 255 for each of R, G, B
    pub fn rgb(&self) -> (f64, f64, f64) {
        (self.r, self.g, self.b)
    }

    /// The RGBA information of this color as 0.0-1.0 range floats representing
    /// proportions of 255 for each of R, G, B, A
    pub fn rgba(&self) -> (f64, f64, f64, f64) {
        (self.r, self.g, self.b, self.a)
    }

    /// Render this color as a #RRGGBB hew color string
    pub fn as_rgb_hex_string(&self) -> String {
        format!("#{:x}", self.rgb_u32())
    }

    /// 0xRRGGBB representation of this Color (no alpha information)
    pub fn rgb_u32(&self) -> u32 {
        _f2u!(self.r, 16) + _f2u!(self.g, 8) + _f2u!(self.b, 0)
    }

    /// 0xRRGGBBAA representation of this Color
    pub fn rgba_u32(&self) -> u32 {
        _f2u!(self.r, 24) + _f2u!(self.g, 16) + _f2u!(self.b, 8) + _f2u!(self.a, 0)
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        Self::new_from_hex(hex)
    }
}

impl From<(f64, f64, f64)> for Color {
    fn from(rgb: (f64, f64, f64)) -> Self {
        let (r, g, b) = rgb;
        Self { r, g, b, a: 1.0 }
    }
}

impl From<(f64, f64, f64, f64)> for Color {
    fn from(rgba: (f64, f64, f64, f64)) -> Self {
        let (r, g, b, a) = rgba;
        Self { r, g, b, a }
    }
}

impl TryFrom<String> for Color {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        (&s[..]).try_into()
    }
}

impl TryFrom<&str> for Color {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        let hex = u32::from_str_radix(s.strip_prefix('#').unwrap_or(s), 16)?;

        if s.len() == 7 {
            Ok(Self::new_from_hex((hex << 8) + 0xFF))
        } else if s.len() == 9 {
            Ok(Self::new_from_hex(hex))
        } else {
            Err(Error::InvalidHexColor { hex_code: s.into() })
        }
    }
}
