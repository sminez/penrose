//! Traits and utilities for rendering custom windows
//!
//! The traits and related structs in this module provide a way for rendering and managing simple
//! graphical applications within Penrose itself. While definitely not what you would want to use
//! for writing a full GUI application, the [Draw] and [DrawContext] traits are enough for setting
//! up simple text based UI elements such as status bars and menus.
use crate::{
    common::{geometry::Region, Xid},
    xconnection::{WinType, XClientHandler, XClientProperties, XKeyboardHandler},
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, convert::TryInto};

pub mod bar;
pub mod widget;

#[doc(inline)]
pub use bar::*;

#[doc(inline)]
pub use widget::{HookableWidget, KeyboardControlled, Widget};

/// Enum to store the various ways that operations can fail when rendering windows
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A hex literal provided to create a [Color] was not RGB / RGBA
    #[error("Invalid Hex color code: {0}")]
    InvalidHexColor(String),

    /// A string hex code was invalid as a hex literal
    #[error("Invalid Hex color code")]
    ParseInt(#[from] std::num::ParseIntError),

    /// A generic error type for use in user code when needing to construct
    /// a simple [DrawError].
    #[error("Unhandled error: {0}")]
    Raw(String),

    /// An attempt was made to use a font that had not beed registered
    #[error("'{0}' is has not been registered as a font")]
    UnknownFont(String),

    /// Wrapper around XCB implementation errors for [draw][crate::draw] traits
    #[cfg(feature = "xcb")]
    #[error(transparent)]
    Xcb(#[from] crate::xcb::Error),

    /// Something went wrong when communicating with the X server
    #[error(transparent)]
    X(#[from] crate::xconnection::Error),

    /// An attempt to use the cairo C API failed when using an XCB implementation
    /// of [Draw] or [DrawContext]
    #[cfg(feature = "xcb")]
    #[error("Error calling Cairo API: {0}")]
    Cairo(#[from] cairo::Error),
}

/// Result type for fallible methods on [Draw] and [DrawContext]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
/// A set of styling options for a text string
pub struct TextStyle {
    /// Font name to use for rendering
    pub font: String,
    /// Point size to render the font at
    pub point_size: i32,
    /// Foreground color in 0xRRGGBB format
    pub fg: Color,
    /// Optional background color in 0xRRGGBB format (default to current background if None)
    pub bg: Option<Color>,
    /// Pixel padding around this piece of text
    pub padding: (f64, f64),
}

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
        let hex = u32::from_str_radix(s.strip_prefix('#').unwrap_or(&s), 16)?;

        if s.len() == 7 {
            Ok(Self::new_from_hex((hex << 8) + 0xFF))
        } else if s.len() == 9 {
            Ok(Self::new_from_hex(hex))
        } else {
            Err(Error::InvalidHexColor(s.into()))
        }
    }
}

/// A simple drawing abstraction
///
/// `Draw` is not intended for use in writing full GUI interfaces, rather it is a simple
/// abstraction layer to allow for the creation of minimal UI elements such as status bars, menus
/// and dialogs. Each `Draw` should also provide an acompanying `DrawContext` that impl that is
/// used by consumers (such as the status bar) for actually drawing to the screen, which the parent
/// `Draw` is responsible for resource management and mapping / unmapping the created windows.
pub trait Draw: XClientHandler + XClientProperties {
    /// The type of drawing context used for drawing
    type Ctx: DrawContext;

    /// Create a new client window with a canvas for drawing
    fn new_window(&mut self, ty: WinType, r: Region, managed: bool) -> Result<Xid>;
    /// Get the size of the target screen in pixels
    fn screen_sizes(&self) -> Result<Vec<Region>>;
    /// Register a font by name for later use
    fn register_font(&mut self, font_name: &str);
    /// Get a new [DrawContext] for the target window
    fn context_for(&self, id: Xid) -> Result<Self::Ctx>;
    /// Get a new temporary [DrawContext] that will be destroyed when dropped
    fn temp_context(&self, w: u32, h: u32) -> Result<Self::Ctx>;
    /// Flush pending actions
    fn flush(&self, id: Xid) -> Result<()>;
}

/// A [Draw] that can return the [KeyPress][1] events from the user for its windows
///
/// [1]: crate::core::bindings::KeyPress
pub trait KeyPressDraw: Draw + XKeyboardHandler {}

impl<T> KeyPressDraw for T where T: Draw + XKeyboardHandler {}

/// Used for simple drawing to the screen
pub trait DrawContext {
    /// Set the active font, must have been registered on the partent Draw
    fn font(&mut self, font_name: &str, point_size: i32) -> Result<()>;
    /// Set the color used for subsequent drawing operations
    fn color(&mut self, color: &Color);
    /// Clears the context
    fn clear(&mut self) -> Result<()>;
    /// Translate this context by (dx, dy) from its current position
    fn translate(&self, dx: f64, dy: f64);
    /// Set the x offset for this context absolutely
    fn set_x_offset(&self, x: f64);
    /// Set the y offset for this context absolutely
    fn set_y_offset(&self, y: f64);
    /// Draw a filled rectangle using the current color
    fn rectangle(&self, x: f64, y: f64, w: f64, h: f64) -> Result<()>;
    /// Render 's' using the current font with the supplied padding. returns the extent taken
    /// up by the rendered text
    fn text(&self, s: &str, h_offset: f64, padding: (f64, f64)) -> Result<(f64, f64)>;
    /// Determine the pixel width of a given piece of text using the current font
    fn text_extent(&self, s: &str) -> Result<(f64, f64)>;
    /// Flush pending actions
    fn flush(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;
    use std::convert::TryFrom;

    #[test_case(0x00000000, (0.0, 0.0, 0.0, 0.0); "black")]
    #[test_case(0x000000FF, (0.0, 0.0, 0.0, 1.0); "black alpha")]
    #[test_case(0xFFFFFFFF, (1.0, 1.0, 1.0, 1.0); "white")]
    #[test_case(0xFF0000FF, (1.0, 0.0, 0.0, 1.0); "red")]
    #[test_case(0x00FF00FF, (0.0, 1.0, 0.0, 1.0); "green")]
    #[test_case(0x0000FFFF, (0.0, 0.0, 1.0, 1.0); "blue")]
    #[test]
    fn color_from_hex_rgba(hex: u32, floats: (f64, f64, f64, f64)) {
        assert_eq!(Color::new_from_hex(hex), Color::from(floats));
    }

    #[test_case("#FFFF00FF", (1.0, 1.0, 0.0, 1.0); "alpha 1")]
    #[test_case("#FFFF0000", (1.0, 1.0, 0.0, 0.0); "alpha 0")]
    #[test]
    fn color_from_str_or_string(s: &str, floats: (f64, f64, f64, f64)) {
        assert_eq!(Color::try_from(s).unwrap(), Color::from(floats));
        assert_eq!(Color::try_from(s.to_string()).unwrap(), Color::from(floats));
    }

    #[test_case("#000000", (0.0, 0.0, 0.0, 1.0); "black")]
    #[test_case("#FFFFFF", (1.0, 1.0, 1.0, 1.0); "white")]
    #[test_case("#FF0000", (1.0, 0.0, 0.0, 1.0); "red")]
    #[test_case("#00FF00", (0.0, 1.0, 0.0, 1.0); "green")]
    #[test_case("#0000FF", (0.0, 0.0, 1.0, 1.0); "blue")]
    #[test]
    fn color_from_str_or_string_no_alpha(s: &str, floats: (f64, f64, f64, f64)) {
        assert_eq!(Color::try_from(s).unwrap(), Color::from(floats));
        assert_eq!(Color::try_from(s.to_string()).unwrap(), Color::from(floats));
    }

    #[test_case("#000000", 0x000000; "black")]
    #[test_case("#FFFFFF", 0xFFFFFF; "white")]
    #[test_case("#FF0000", 0xFF0000; "red")]
    #[test_case("#00FF00", 0x00FF00; "green")]
    #[test_case("#0000FF", 0x0000FF; "blue")]
    #[test]
    fn color_rgb_u32(s: &str, expected: u32) {
        assert_eq!(Color::try_from(s).unwrap().rgb_u32(), expected);
    }

    #[test_case("#00000000", 0x00000000; "black")]
    #[test_case("#FFFFFF00", 0xFFFFFF00; "white")]
    #[test_case("#FF000000", 0xFF000000; "red")]
    #[test_case("#00FF0000", 0x00FF0000; "green")]
    #[test_case("#0000FF00", 0x0000FF00; "blue")]
    #[test]
    fn color_rgba_u32(s: &str, expected: u32) {
        assert_eq!(Color::try_from(s).unwrap().rgba_u32(), expected);
    }
}
