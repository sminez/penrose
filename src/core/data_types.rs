//! Simple data types and enums
use crate::{core::xconnection::Atom, PenroseError, Result};

/// Output of a Layout function: the new position a window should take
pub type ResizeAction = (WinId, Option<Region>);

/// An X window ID
pub type WinId = u32;

/// A client propert value that can be set.
///
/// Variants correspond to the X property types being set.
#[derive(Clone, Copy, Debug)]
pub enum PropVal<'a> {
    /// A slice of interned [Atom] values
    Atom(&'a [u32]),
    /// A slice of cardinal u32s
    Cardinal(&'a [u32]),
    /// A string valued property
    Str(&'a str),
    /// One or more [WinId] values
    Window(&'a [WinId]),
}

/// A window type to be specified when creating a new window in the X server
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WinType {
    /// A simple hidden stub window for facilitating other API calls
    CheckWin,
    /// A window that receives input only (not queryable)
    InputOnly,
    /// A regular window. The [Atom] passed should be a
    /// valid _NET_WM_WINDOW_TYPE (this is not enforced)
    InputOutput(Atom),
}

/// Config options for X windows (not all are currently implemented)
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WinConfig {
    /// The border width in pixels
    BorderPx(u32),
    /// Absolute size and position on the screen as a [Region]
    Position(Region),
    /// Mark this window as stacking on top of its peers
    StackAbove,
}

impl From<&WinConfig> for Vec<(u16, u32)> {
    fn from(w: &WinConfig) -> Vec<(u16, u32)> {
        match w {
            WinConfig::BorderPx(px) => vec![(xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, *px)],
            WinConfig::Position(region) => {
                let (x, y, w, h) = region.values();
                vec![
                    (xcb::CONFIG_WINDOW_X as u16, x),
                    (xcb::CONFIG_WINDOW_Y as u16, y),
                    (xcb::CONFIG_WINDOW_WIDTH as u16, w),
                    (xcb::CONFIG_WINDOW_HEIGHT as u16, h),
                ]
            }
            WinConfig::StackAbove => {
                vec![(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)]
            }
        }
    }
}

/// Window attributes for an X11 client window (not all are curently implemented)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug)]
pub enum WinAttr {
    /// Border color as an argb hex value
    BorderColor(u32),
    /// Set the pre-defined client event mask
    ClientEventMask,
    /// Set the pre-defined root event mask
    RootEventMask,
}

impl From<&WinAttr> for Vec<(u32, u32)> {
    fn from(w: &WinAttr) -> Vec<(u32, u32)> {
        let client_event_mask = xcb::EVENT_MASK_ENTER_WINDOW
            | xcb::EVENT_MASK_LEAVE_WINDOW
            | xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_STRUCTURE_NOTIFY;

        let root_event_mask = xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xcb::EVENT_MASK_BUTTON_MOTION;

        match w {
            WinAttr::BorderColor(c) => vec![(xcb::CW_BORDER_PIXEL, *c)],
            WinAttr::ClientEventMask => vec![(xcb::CW_EVENT_MASK, client_event_mask)],
            WinAttr::RootEventMask => vec![(xcb::CW_EVENT_MASK, root_event_mask)],
        }
    }
}

/// An x,y coordinate pair
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub struct Point {
    /// An absolute x coordinate relative to the root window
    pub x: u32,
    /// An absolute y coordinate relative to the root window
    pub y: u32,
}

impl Point {
    /// Create a new Point.
    pub fn new(x: u32, y: u32) -> Point {
        Point { x, y }
    }
}

/* Argument enums */

/// Increment / decrement a value
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone)]
pub enum Change {
    /// increase the value
    More,
    /// decrease the value, possibly clamping
    Less,
}

/// X window border kind
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug)]
pub enum Border {
    /// window is urgent
    Urgent,
    /// window currently has focus
    Focused,
    /// window does not have focus
    Unfocused,
}

/// An X window / screen position: top left corner + extent
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Region {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Region {
    /// Create a new Region.
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Region {
        Region { x, y, w, h }
    }

    /// Destructure this Region into its component values (x, y, w, h).
    pub fn values(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    /// Divides this region into two columns where the first has the given width.
    pub fn split_at_width(&self, new_width: u32) -> Result<(Region, Region)> {
        if new_width > self.w {
            Err(PenroseError::SplitError(new_width, self.w))
        } else {
            Ok((
                Region {
                    w: new_width,
                    ..*self
                },
                Region {
                    x: self.x + new_width,
                    w: self.w - new_width,
                    ..*self
                },
            ))
        }
    }

    /// Divides this region into two rows where the first has the given height.
    pub fn split_at_height(&self, new_height: u32) -> Result<(Region, Region)> {
        if new_height > self.h {
            Err(PenroseError::SplitError(new_height, self.h))
        } else {
            Ok((
                Region {
                    h: new_height,
                    ..*self
                },
                Region {
                    y: self.y + new_height,
                    h: self.h - new_height,
                    ..*self
                },
            ))
        }
    }
}
