//! Simple data types and enums
use crate::{
    core::xconnection::{Atom, Xid},
    Result,
};

/// Output of a Layout function: the new position a window should take
pub type ResizeAction = (Xid, Option<Region>);

/// An X window ID
pub type WinId = u32;

/// A window type to be specified when creating a new window in the X server
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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

/// A relative position along the horizontal and vertical axes
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RelativePosition {
    /// Left of the current position
    Left,
    /// Right of the current position
    Right,
    /// Above the current position
    Above,
    /// Below the current position
    Below,
}

/// An x,y coordinate pair
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Point {
    /// An absolute x coordinate relative to the root window
    pub x: u32,
    /// An absolute y coordinate relative to the root window
    pub y: u32,
}

impl Point {
    /// Create a new Point.
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

impl Default for Point {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/* Argument enums */

/// Increment / decrement a value
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Change {
    /// increase the value
    More,
    /// decrease the value, possibly clamping
    Less,
}

/// X window border kind
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Region {
    /// The x-coordinate of the top left corner of this region
    pub x: u32,
    /// The y-coordinate of the top left corner of this region
    pub y: u32,
    /// The width of this region
    pub w: u32,
    /// The height of this region
    pub h: u32,
}

impl Default for Region {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl Region {
    /// Create a new Region.
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Region {
        Region { x, y, w, h }
    }

    /// Destructure this Region into its component values (x, y, w, h).
    ///
    /// # Examples
    ///
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
    ///
    /// # Examples
    ///
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
    ///
    /// # Examples
    ///
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
    ///
    /// # Examples
    ///
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

    /// Check whether this Region contains `p`
    ///
    /// # Examples
    ///
    /// ```
    /// use penrose::core::data_types::{Point, Region};
    ///
    /// let r1 = Region::new(10, 10, 50, 50);
    ///
    /// assert!(r1.contains_point(&Point::new(30, 20)));
    /// assert!(!r1.contains_point(&Point::new(0, 0)));
    /// ```
    pub fn contains_point(&self, p: &Point) -> bool {
        (self.x..(self.x + self.w)).contains(&p.x) && (self.y..(self.y + self.h)).contains(&p.y)
    }

    /// Center this region inside of `enclosing`.
    ///
    /// # Errors
    /// Fails if this Region can not fit inside of `enclosing`
    ///
    /// # Examples
    ///
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r1 = Region::new(10, 10, 50, 60);
    /// let r2 = Region::new(0, 0, 100, 100);
    ///
    /// let centered = r1.centered_in(&r2);
    /// assert!(centered.is_ok());
    /// assert_eq!(centered.unwrap(), Region::new(25, 20, 50, 60));
    ///
    /// let too_big = r2.centered_in(&r1);
    /// assert!(too_big.is_err());
    /// ```
    pub fn centered_in(&self, enclosing: &Region) -> Result<Self> {
        if !enclosing.contains(self) {
            return Err(perror!(
                "enclosing does not conatain self: {:?} {:?}",
                enclosing,
                self
            ));
        }

        Ok(Self {
            x: enclosing.x + ((enclosing.w - self.w) / 2),
            y: enclosing.y + ((enclosing.h - self.h) / 2),
            ..*self
        })
    }

    /// Split this `Region` into evenly sized rows.
    ///
    /// # Examples
    ///
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(0, 0, 100, 100);
    ///
    /// let regions = r.as_rows(2);
    ///
    /// assert_eq!(regions.len(), 2);
    /// assert_eq!(regions[0], Region::new(0, 0, 100, 50));
    /// assert_eq!(regions[1], Region::new(0, 50, 100, 50));
    /// ```
    pub fn as_rows(&self, n_rows: u32) -> Vec<Region> {
        if n_rows <= 1 {
            return vec![*self];
        }
        let h = self.h / n_rows as u32;
        (0..n_rows)
            .map(|n| Region::new(self.x, (self.y + n as u32 * h) as u32, self.w, h))
            .collect()
    }

    /// Split this `Region` into evenly sized columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(0, 0, 100, 100);
    ///
    /// let regions = r.as_columns(2);
    ///
    /// assert_eq!(regions.len(), 2);
    /// assert_eq!(regions[0], Region::new(0, 0, 50, 100));
    /// assert_eq!(regions[1], Region::new(50, 0, 50, 100));
    /// ```
    pub fn as_columns(&self, n_columns: u32) -> Vec<Region> {
        if n_columns <= 1 {
            return vec![*self];
        }
        let w = self.w / n_columns as u32;
        (0..n_columns)
            .map(|n| Region::new((self.x + n as u32 * w) as u32, self.y, w, self.h))
            .collect()
    }

    /// Divides this region into two columns where the first has the given width.
    ///
    /// # Errors
    /// Fails if the requested split point is not contained within `self`
    ///
    /// # Examples
    ///
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(10, 10, 50, 60);
    /// let (r1, r2) = r.split_at_width(30).unwrap();
    ///
    /// assert_eq!(r1, Region::new(10, 10, 30, 60));
    /// assert_eq!(r2, Region::new(40, 10, 20, 60));
    ///
    /// assert!(r.split_at_width(100).is_err());
    /// ```
    pub fn split_at_width(&self, new_width: u32) -> Result<(Self, Self)> {
        if new_width > self.w {
            Err(perror!(
                "Region split is out of range: {} >= {}",
                new_width,
                self.w
            ))
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
    ///
    /// # Examples
    ///
    /// ```
    /// use penrose::core::data_types::Region;
    ///
    /// let r = Region::new(10, 10, 50, 60);
    /// let (r1, r2) = r.split_at_height(40).unwrap();
    ///
    /// assert_eq!(r1, Region::new(10, 10, 50, 40));
    /// assert_eq!(r2, Region::new(10, 50, 50, 20));
    ///
    /// assert!(r.split_at_height(100).is_err());
    /// ```
    pub fn split_at_height(&self, new_height: u32) -> Result<(Self, Self)> {
        if new_height > self.h {
            Err(perror!(
                "Region split is out of range: {} >= {}",
                new_height,
                self.h
            ))
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
