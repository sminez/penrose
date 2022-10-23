use penrose::{x::XConn, Color, Xid};

mod bar;
mod core;
mod widgets;

pub use crate::core::{Context, Draw, TextStyle};
pub use bar::{Position, StatusBar};
pub use widgets::{Text, Widget};

use crate::widgets::{ActiveWindowName, CurrentLayout, RootWindowName, Workspaces};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Cairo(#[from] cairo::Error),

    #[error("Invalid Hex color code: {code}")]
    InvalidHexColor { code: String },

    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),

    #[error(transparent)]
    Penrose(#[from] penrose::Error),

    #[error("unable to create pango layout")]
    UnableToCreateLayout,

    #[error("no cairo surface for {id}")]
    UnintialisedSurface { id: Xid },

    #[error("'{font}' is has not been registered as a font")]
    UnknownFont { font: String },
}

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
        &[&style.font],
        vec![
            Box::new(Workspaces::new(style, highlight, empty_ws)),
            Box::new(CurrentLayout::new(style)),
            Box::new(ActiveWindowName::new(
                max_active_window_chars,
                &TextStyle {
                    bg: Some(highlight),
                    padding: (6.0, 4.0),
                    ..style.clone()
                },
                true,
                false,
            )),
            Box::new(RootWindowName::new(
                &TextStyle {
                    padding: (4.0, 2.0),
                    ..style.clone()
                },
                false,
                true,
            )),
        ],
    )
}
