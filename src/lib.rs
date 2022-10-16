//! # Penrose: a library for building your very own tiling window manager
pub mod actions;
pub mod bindings;
pub mod core;
pub mod extensions;
pub mod geometry;
pub mod handle;
pub mod hooks;
pub mod layout;
pub mod macros;
pub mod pure;
pub mod util;
pub mod x;
pub mod xcb; // TODO: should be feature flagged

pub use crate::core::Xid;
pub use geometry::{Point, Rect};
pub use pure::{Position, Screen, Stack, StackSet, Workspace};

#[derive(Debug, thiserror::Error)]
pub enum Error {
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

    // FIXME: feature flag
    #[error("Unable to connect to the X server via XCB")]
    XcbConnection(#[from] ::xcb::ConnError),

    // FIXME: feature flag
    #[error("X11 error: error seq={0}, code={1}, xid={2}, request: {3}:{4}")]
    X11Error(u16, u8, u32, u8, u16),

    // FIXME: feature flag
    #[error("Error making xcb query: {0:?}")]
    XcbKnown(crate::xcb::error::XErrorCode),

    // FIXME: feature flag
    #[error("Expected XCB response type to be one of {expected:?}, got {received}")]
    XcbUnexpectedResponseType { expected: Vec<u8>, received: u8 },

    // FIXME: feature flag
    #[error("Unknown error making xcb query: error_code={0} response_type={1}")]
    XcbUnknown(u8, u8),
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
