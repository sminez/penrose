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
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// // In practice, this will be a region your code is receiving: not one you create
    /// let r = Region::new(10, 20, 30, 40);
    ///
    /// assert_eq!(r.values(), (10, 20, 30, 40));
    /// ```
    pub fn values(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    /// Create a new [Region] with width equal to `factor` x `self.w`
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(10, 20, 30, 40);
    ///
    /// assert_eq!(r.scale_w(1.5), Region::new(10, 20, 45, 40));
    /// assert_eq!(r.scale_w(0.5), Region::new(10, 20, 15, 40));
    /// ```
    pub fn scale_w(&self, factor: f64) -> Self {
        Self {
            w: (self.w as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Create a new [Region] with height equal to `factor` x `self.h`
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(10, 20, 30, 40);
    ///
    /// assert_eq!(r.scale_h(1.5), Region::new(10, 20, 30, 60));
    /// assert_eq!(r.scale_h(0.5), Region::new(10, 20, 30, 20));
    /// ```
    pub fn scale_h(&self, factor: f64) -> Self {
        Self {
            h: (self.h as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Check whether this Region contains `other` as a sub-Region
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r1 = Region::new(10, 10, 50, 50);
    /// let r2 = Region::new(0, 0, 100, 100);
    ///
    /// assert!(r2.contains(&r1));
    /// assert!(!r1.contains(&r2));
    /// ```
    pub fn contains(&self, other: &Region) -> bool {
        match other {
            Region { x, .. } if *x < self.x => false,
            Region { x, w, .. } if (*x + *w) > (self.x + self.w) => false,
            Region { y, .. } if *y < self.y => false,
            Region { y, h, .. } if (*y + *h) > (self.y + self.h) => false,
            _ => true,
        }
    }

    /// Center this region inside of `enclosing`.
    ///
    /// # Errors
    /// Fails if this Region can not fit inside of `enclosing`
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r1 = Region::new(10, 10, 50, 60);
    /// let r2 = Region::new(0, 0, 100, 100);
    ///
    /// let centered = r1.centered_in(&r2);
    /// assert!(centered.is_ok());
    /// assert_eq!(centered.unwrap(), Region::new(25, 30, 50, 60));
    ///
    /// let too_big = r2.centered_in(&r1);
    /// assert!(too_big.is_err());
    /// ```
    pub fn centered_in(&self, enclosing: &Region) -> Result<Self> {
        if !enclosing.contains(self) {
            return Err(PenroseError::Raw(format!(
                "enclosing does not conatain self: {:?} {:?}",
                enclosing, self
            )));
        }

        Ok(Self {
            x: enclosing.x + (self.w / 2),
            y: enclosing.y + (self.h / 2),
            ..*self
        })
    }

    /// Divides this region into two columns where the first has the given width.
    ///
    /// # Errors
    /// Fails if the requested split point is not contained within `self`
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(10, 10, 50, 60);
    /// let (r1, r2) = r.split_at_width(30).unwrap();
    ///
    /// assert_eq!(r1, Region::new(10,10,30,60));
    /// assert_eq!(r2, Region::new(40,10,20,60));
    ///
    /// assert!(r.split_at_width(100).is_err());
    /// ```
    pub fn split_at_width(&self, new_width: u32) -> Result<(Self, Self)> {
        if new_width > self.w {
            Err(PenroseError::Raw(format!(
                "Region split is out of range: {} >= {}",
                new_width, self.w
            )))
        } else {
            Ok((
                Self {
                    w: new_width,
                    ..*self
                },
                Self {
                    x: self.x + new_width,
                    w: self.w - new_width,
                    ..*self
                },
            ))
        }
    }

    /// Divides this region into two rows where the first has the given height.
    ///
    /// # Errors
    /// Fails if the requested split point is not contained within `self`
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(10, 10, 50, 60);
    /// let (r1, r2) = r.split_at_height(40).unwrap();
    ///
    /// assert_eq!(r1, Region::new(10,10,50,40));
    /// assert_eq!(r2, Region::new(10,50,50,20));
    ///
    /// assert!(r.split_at_height(100).is_err());
    /// ```
    pub fn split_at_height(&self, new_height: u32) -> Result<(Self, Self)> {
        if new_height > self.h {
            Err(PenroseError::Raw(format!(
                "Region split is out of range: {} >= {}",
                new_height, self.h
            )))
        } else {
            Ok((
                Self {
                    h: new_height,
                    ..*self
                },
                Self {
                    y: self.y + new_height,
                    h: self.h - new_height,
                    ..*self
                },
            ))
        }
    }
}
