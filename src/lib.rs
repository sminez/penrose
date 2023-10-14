//! # Penrose: a library for building your very own tiling window manager
//!
//! Penrose is inspired by similar projects such as [dwm][1], [xmonad][2] and [qtile][3] which
//! allow you to configure your window manager in code and compile it for your system. It is most
//! similar to `xmonad` in that it is more of a library for building a window manager (with low
//! level details taken care of for you) rather than a minimal window manager that you edit and
//! patch directly (such as `dwm`). Penrose strives to be as simple as possible in its
//! implementation in order to make the guts of the window manager easier to understand. Given the
//! nature of what this involves, this is not always possible but effort has been made to keep the
//! source readable and with relatively free of magic.
//!
//!
//! ## Using Penrose
//!
//! Penrose itself is not a binary application that you can build, install and run. You need to
//! write your own **main.rs** as a rust binary crate that uses Penrose as a dependency to set up,
//! configure and run your very own window manager exactly how you want it. In short, you *will*
//! need to write some code and you *will* need to know Rust to some degree.
//!
//! For learning rust itself, there are some fantastic official [guides][4] available on
//! rust-lang.org and if you are sticking to using the out of the box functionality provided
//! by the penrose crate, working through [The Rust Book][5] before diving into penrose should be more
//! than enough to get you started.
//!
//! On GitHub you can find up to date [examples][6] of how to set up and configure a window manager
//! using penrose, ranging from bare bones minimal to custom extensions and hooks.
//!
//! > **NOTE**: in order to use the xcb implementation of penrose, you will need to install the C
//! > libraries that are dependencies (namely xcb, Cairo and Pango).
//!
//!
//! ## Digging into the API
//!
//! The suggested reading order for getting to grips with the penrose API is to first look at the
//! [pure][7] data structures that represent the logical state of your window manager before digging
//! in to the [core][8] module which contains the majority of the functionality you are likely to
//! want to work with. If you are interested in the lower level X11 interactions (or need to make
//! requests to the X server directly) you should check out the [x][9] module and its associated
//! traits. To add functionality and flexability to your window manager, there are the [builtin][10]
//! and [extensions][11] modules which offer capabilities built on top of the rest of penrose.
//!
//! [1]: https://dwm.suckless.org/
//! [2]: https://xmonad.org/
//! [3]: http://www.qtile.org/
//! [4]: https://www.rust-lang.org/learn
//! [5]: https://doc.rust-lang.org/book/
//! [6]: https://github.com/sminez/penrose/tree/develop/examples
//! [7]: crate::pure
//! [8]: crate::core
//! [9]: crate::x
//! [10]: crate::builtin
//! [11]: crate::extensions
#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::style,
    future_incompatible,
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    rustdoc::all,
    clippy::undocumented_unsafe_blocks
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg",
    issue_tracker_base_url = "https://github.com/sminez/penrose/issues/"
)]

#[cfg(feature = "x11rb")]
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
mod macros;
pub mod pure;
pub mod util;
pub mod x;
#[cfg(feature = "x11rb")]
pub mod x11rb;

#[doc(inline)]
pub use crate::core::Xid;

/// Error variants from the core penrose library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An operation requiring the client to be on a screen was requested on a client window that
    /// is not currently visible
    #[error("Client {0} is not currently visible")]
    ClientIsNotVisible(Xid),

    /// A custom error message from user code or extensions
    #[error("{0}")]
    Custom(String),

    /// There were not enough workspaces to cover the number of connected screens
    #[error("Only {n_ws} workspaces were provided but at least {n_screens} are required")]
    InsufficientWorkspaces {
        /// Number of provided workspaces
        n_ws: usize,
        /// Number of connected screens
        n_screens: usize,
    },

    /// Data received as part of a client message had an invalid format
    #[error("invalid client message data: format={format}")]
    InvalidClientMessage {
        /// The format received
        format: u8,
    },

    /// Attempt to create a `Color` from an invalid hex string
    #[error("Invalid Hex color code: '{hex_code}'")]
    InvalidHexColor {
        /// The string that was used
        hex_code: String,
    },

    /// A window hints message was received but unable to be parsed
    #[error("Invalid window hints message: {reason}")]
    InvalidHints {
        /// Why parsing failed
        reason: String,
    },

    /// IO error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Invalid UTF8 encoded string
    #[error(transparent)]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// Data received from the X server when requesting a window property was invalid
    #[error("{ty} property '{prop}' for {id} contained invalid data")]
    InvalidPropertyData {
        /// The window that was queried
        id: Xid,
        /// The type of property that was queried
        ty: String,
        /// The name of the property that was queried
        prop: String,
    },

    /// Duplicate tags were provided for one or more workspaces
    #[error("The following tags have been used multiple times for different workspaces: {tags:?}")]
    NonUniqueTags {
        /// The set of non-unique tags
        tags: Vec<String>,
    },

    /// Penrose is running without any screens to connect to
    #[error("There are no screens available")]
    NoScreens,

    /// ParseIntError
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// There was a problem initialising randr
    #[error("Error initialising randr: {0}")]
    Randr(String),

    /// An operation was requested on a client window that is unknown
    #[error("Client {0} is not in found")]
    UnknownClient(Xid),

    /// A keybinding has been specified for an unknown key name for this machine.
    #[error("'{name}' is not a known key name")]
    UnknownKeyName {
        /// The name of the unknown key
        name: String,
    },

    /// An unknown character has been used to specify a modifier key
    #[error("'{name}' is not a known modifier key")]
    UnknownModifier {
        /// The unrecognised modifier name
        name: String,
    },

    /// An unknown mouse button was pressed
    #[error("{button} is not a supported mouse button")]
    UnknownMouseButton {
        /// The button ID that was pressed
        button: u8,
    },

    /// An attempt was made to fetch a state extension for a type that has not been stored
    #[error("{type_id:?} was requested as a state extension but not found")]
    UnknownStateExtension {
        /// The type ID of the type that was requested
        type_id: TypeId,
    },

    // TODO: These backend specific errors should be abstracted out to a
    //       set of common error variants that they can be mapped to without
    //       needing to extend the enum conditionally when flags are enabled
    /// An error that occurred while connecting to an X11 server
    #[cfg(feature = "x11rb")]
    #[error(transparent)]
    X11rbConnect(#[from] ConnectError),

    /// An error that occurred on an already established X11 connection
    #[cfg(feature = "x11rb")]
    #[error(transparent)]
    X11rbConnection(#[from] ConnectionError),

    /// An error that occurred with some request.
    #[cfg(feature = "x11rb")]
    #[error(transparent)]
    X11rbReplyError(#[from] ReplyError),

    /// An error caused by some request or by the exhaustion of IDs.
    #[cfg(feature = "x11rb")]
    #[error(transparent)]
    X11rbReplyOrIdError(#[from] ReplyOrIdError),

    /// Representation of an X11 error packet that was sent by the server.
    #[cfg(feature = "x11rb")]
    #[error("X11 error: {0:?}")]
    X11rbX11Error(X11Error),
}

/// A Result where the error type is a penrose [Error]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A simple RGBA based color
pub struct Color {
    rgba_hex: u32,
}

impl Color {
    /// Create a new Color from a hex encoded u32: 0xRRGGBB or 0xRRGGBBAA
    pub fn new_from_hex(rgba_hex: u32) -> Self {
        Self { rgba_hex }
    }

    /// The RGB information of this color as 0.0-1.0 range floats representing
    /// proportions of 255 for each of R, G, B
    pub fn rgb(&self) -> (f64, f64, f64) {
        let (r, g, b, _) = self.rgba();

        (r, g, b)
    }

    /// The RGBA information of this color as 0.0-1.0 range floats representing
    /// proportions of 255 for each of R, G, B, A
    pub fn rgba(&self) -> (f64, f64, f64, f64) {
        let floats: Vec<f64> = self
            .rgba_hex
            .to_be_bytes()
            .iter()
            .map(|n| *n as f64 / 255.0)
            .collect();

        (floats[0], floats[1], floats[2], floats[3])
    }

    /// Render this color as a #RRGGBB hew color string
    pub fn as_rgb_hex_string(&self) -> String {
        format!("#{:x}", self.rgb_u32())
    }

    /// 0xRRGGBB representation of this Color (no alpha information)
    pub fn rgb_u32(&self) -> u32 {
        self.rgba_hex >> 8
    }

    /// 0xRRGGBBAA representation of this Color
    pub fn rgba_u32(&self) -> u32 {
        self.rgba_hex
    }

    /// 0xAARRGGBB representation of this Color
    pub fn argb_u32(&self) -> u32 {
        ((self.rgba_hex & 0x000000FF) << 24) + (self.rgba_hex >> 8)
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        Self::new_from_hex(hex)
    }
}

macro_rules! _f2u { { $f:expr, $s:expr } => { (($f * 255.0) as u32) << $s } }

impl From<(f64, f64, f64)> for Color {
    fn from(rgb: (f64, f64, f64)) -> Self {
        let (r, g, b) = rgb;
        let rgba_hex = _f2u!(r, 24) + _f2u!(g, 16) + _f2u!(b, 8) + _f2u!(1.0, 0);

        Self { rgba_hex }
    }
}

impl From<(f64, f64, f64, f64)> for Color {
    fn from(rgba: (f64, f64, f64, f64)) -> Self {
        let (r, g, b, a) = rgba;
        let rgba_hex = _f2u!(r, 24) + _f2u!(g, 16) + _f2u!(b, 8) + _f2u!(a, 0);

        Self { rgba_hex }
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
