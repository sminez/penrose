//! # Penrose-ui: a bare bones toolkit for adding UI elements to Penrose
//!
//! ## A note on the intended purpose of this crate
//! Penrose-ui is not intended as a general purpose UI library. It is incredibly minimal in the
//! functionality it provides and is primarily designed to provide a built-in status bar for the
//! [penrose][0] tiling window manager library. While it should be possible to make use of this
//! crate for writing UIs without integrating with penrose, that is certainly not the intended
//! use case and is not fully supported.
//!
//! ## Getting started
//! The main functionality of this crate is provided through the [`Draw`] nad [`Context`] structs
//! which allow for simple graphics rendering backed by the xlib and fontconfig libraries.
//!
//! [0]: https://github.com/sminez/penrose
#![warn(
    clippy::complexity,
    clippy::correctness,
    clippy::style,
    future_incompatible,
    missing_debug_implementations,
    missing_docs,
    rust_2018_idioms,
    rustdoc::all
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg",
    issue_tracker_base_url = "https://github.com/sminez/penrose/issues/"
)]

use penrose::{x::XConn, Color, Xid};
use std::ffi::NulError;

pub mod bar;
pub mod core;

pub use crate::core::{Context, Draw, TextStyle};
pub use bar::{Position, StatusBar};

use bar::widgets::{ActiveWindowName, CurrentLayout, RootWindowName, Workspaces};

/// Error variants from penrose_ui library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Creation of a [`Color`] from a string hex code was invalid
    #[error("Invalid Hex color code: {code}")]
    InvalidHexColor {
        /// The invalid string that was intended as a color hex code
        code: String,
    },

    /// The specified character can not be rendered by any font on this system
    #[error("Unable to find a fallback font for '{0}'")]
    NoFallbackFontForChar(char),

    /// A string being passed to underlying C APIs contained an internal null byte
    #[error(transparent)]
    NulError(#[from] NulError),

    /// Unable to parse an integer from a provided string.
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    /// An error was returned from the [`XConn`] when interacting with the X server
    #[error(transparent)]
    Penrose(#[from] penrose::Error),

    /// Unable to allocate a requested color
    #[error("Unable to allocate the requested color using Xft")]
    UnableToAllocateColor,

    /// Unable to open a requested font
    #[error("Unable to open '{0}' as a font using Xft")]
    UnableToOpenFont(String),

    /// Unable to open a font using an Xft font pattern
    #[error("Unable to open font from FcPattern using Xft")]
    UnableToOpenFontPattern,

    /// Unable to parse an Xft font pattern
    #[error("Unable to parse '{0}' as an Xft font patten")]
    UnableToParseFontPattern(String),

    /// An attempt was made to work with a surface for a window that was not initialised
    /// by the [`Draw`] instance being used.
    #[error("no surface for {id}")]
    UnintialisedSurface {
        /// The window id requested
        id: Xid,
    },
}

/// A Result where the error type is a penrose_ui [`Error`]
pub type Result<T> = std::result::Result<T, Error>;

/// Create a default dwm style status bar that displays content pulled from the
/// WM_NAME property of the root window.
pub fn status_bar<X: XConn>(
    height: u32,
    style: &TextStyle,
    highlight: impl Into<Color>,
    empty_ws: impl Into<Color>,
    position: Position,
) -> Result<StatusBar<X>> {
    let max_active_window_chars = 80;
    let highlight = highlight.into();

    StatusBar::try_new(
        position,
        height,
        style.bg.unwrap_or_else(|| 0x000000.into()),
        &style.font,
        style.point_size,
        vec![
            Box::new(Workspaces::new(style, highlight, empty_ws)),
            Box::new(CurrentLayout::new(style)),
            Box::new(ActiveWindowName::new(
                max_active_window_chars,
                &TextStyle {
                    bg: Some(highlight),
                    padding: (6, 4),
                    ..style.clone()
                },
                true,
                false,
            )),
            Box::new(RootWindowName::new(
                &TextStyle {
                    padding: (4, 2),
                    ..style.clone()
                },
                false,
                true,
            )),
        ],
    )
}
