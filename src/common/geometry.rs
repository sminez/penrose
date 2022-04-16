use crate::common::Xid;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// An x,y coordinate pair
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

pub type ResizeAction = (Xid, Option<Region>);

/// An X window / screen position: top left corner + extent
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
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

impl Region {
    /// Create a new Region.
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Region {
        Region { x, y, w, h }
    }

    /// Destructure this Region into its component values (x, y, w, h).
    pub fn values(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }

    /// Create a new [Region] with width equal to `factor` x `self.w`
    pub fn scale_w(&self, factor: f64) -> Self {
        Self {
            w: (self.w as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Create a new [Region] with height equal to `factor` x `self.h`
    pub fn scale_h(&self, factor: f64) -> Self {
        Self {
            h: (self.h as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Check whether this Region contains `other` as a sub-Region
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
    pub fn contains_point(&self, p: &Point) -> bool {
        (self.x..(self.x + self.w + 1)).contains(&p.x)
            && (self.y..(self.y + self.h + 1)).contains(&p.y)
    }

    /// Center this region inside of `enclosing`.
    ///
    /// # Errors
    /// Fails if this Region can not fit inside of `enclosing`
    pub fn centered_in(&self, enclosing: &Region) -> Result<Self, String> {
        if self.w > enclosing.w || self.h > enclosing.h {
            return Err(format!(
                "enclosing does not conatain self: {:?} {:?}",
                enclosing, self
            ));
        }

        Ok(Self {
            x: enclosing.x + ((enclosing.w - self.w) / 2),
            y: enclosing.y + ((enclosing.h - self.h) / 2),
            ..*self
        })
    }

    /// Split this `Region` into evenly sized rows.
    pub fn as_rows(&self, n_rows: u32) -> Vec<Region> {
        if n_rows <= 1 {
            return vec![*self];
        }
        let h = self.h / n_rows;
        (0..n_rows)
            .map(|n| Region::new(self.x, (self.y + n as u32 * h) as u32, self.w, h))
            .collect()
    }

    /// Split this `Region` into evenly sized columns.
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
    pub fn split_at_width(&self, new_width: u32) -> Result<(Self, Self), String> {
        if new_width >= self.w {
            Err(format!(
                "Region split is out of range: {} >= {}",
                new_width, self.w
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
    pub fn split_at_height(&self, new_height: u32) -> Result<(Self, Self), String> {
        if new_height >= self.h {
            Err(format!(
                "Region split is out of range: {} >= {}",
                new_height, self.h
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

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(1.5, Region::new(10, 20, 45, 40); "scale up")]
    #[test_case(0.5, Region::new(10, 20, 15, 40); "scale down")]
    #[test_case(1.0, Region::new(10, 20, 30, 40); "unchanged")]
    #[test]
    fn scale_w(factor: f64, expected: Region) {
        let r = Region::new(10, 20, 30, 40);

        assert_eq!(r.scale_w(factor), expected);
    }

    #[test_case(1.5, Region::new(10, 20, 30, 60); "scale up")]
    #[test_case(0.5, Region::new(10, 20, 30, 20); "scale down")]
    #[test_case(1.0, Region::new(10, 20, 30, 40); "unchanged")]
    #[test]
    fn scale_h(factor: f64, expected: Region) {
        let r = Region::new(10, 20, 30, 40);

        assert_eq!(r.scale_h(factor), expected);
    }

    #[test]
    fn contains_region() {
        let r1 = Region::new(10, 10, 50, 50);
        let r2 = Region::new(0, 0, 100, 100);

        assert!(r2.contains(&r1));
        assert!(!r1.contains(&r2));
    }

    #[test_case(Point::new(0, 0), false; "outside")]
    #[test_case(Point::new(30, 20), true; "inside")]
    #[test_case(Point::new(10, 20), true; "top left")]
    #[test_case(Point::new(40, 20), true; "top right")]
    #[test_case(Point::new(10, 60), true; "bottom left")]
    #[test_case(Point::new(40, 60), true; "bottom right")]
    #[test]
    fn contains_point(p: Point, expected: bool) {
        let r = Region::new(10, 20, 30, 40);

        assert_eq!(r.contains_point(&p), expected);
    }

    #[test_case(
        Region::new(0, 0, 10, 10),
        Some(Region::new(5, 5, 10, 10));
        "fits"
    )]
    #[test_case(
        Region::new(10, 10, 10, 10),
        Some(Region::new(5, 5, 10, 10));
        "fits overlaping"
    )]
    #[test_case(
        Region::new(100, 100, 10, 10),
        Some(Region::new(5, 5, 10, 10));
        "fits but not contained"
    )]
    #[test_case(Region::new(0, 0, 100, 100), None; "doesn't fit")]
    #[test]
    fn centered_in(inner: Region, expected: Option<Region>) {
        let outer = Region::new(0, 0, 20, 20);

        let res = inner.centered_in(&outer);

        if let Some(r) = expected {
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), r);
        } else {
            assert!(res.is_err());
        }
    }

    #[test_case(Region::new(0, 0, 100, 100), 1; "simple single")]
    #[test_case(Region::new(0, 0, 100, 100), 4; "simple even")]
    #[test_case(Region::new(0, 0, 100, 100), 7; "simple odd")]
    #[test_case(Region::new(0, 0, 79, 57), 1; "awkward single")]
    #[test_case(Region::new(0, 0, 79, 57), 4; "awkward even")]
    #[test_case(Region::new(0, 0, 79, 57), 7; "awkward odd")]
    #[test]
    fn as_rows(r: Region, n_rows: u32) {
        let regions = r.as_rows(n_rows);
        let h = regions[0].h;

        assert_eq!(regions.len(), n_rows as usize);
        assert!(regions.iter().all(|r| r.h == h));
    }

    #[test_case(Region::new(0, 0, 100, 100), 1; "simple single")]
    #[test_case(Region::new(0, 0, 100, 100), 4; "simple even")]
    #[test_case(Region::new(0, 0, 100, 100), 7; "simple odd")]
    #[test_case(Region::new(0, 0, 79, 57), 1; "awkward single")]
    #[test_case(Region::new(0, 0, 79, 57), 4; "awkward even")]
    #[test_case(Region::new(0, 0, 79, 57), 7; "awkward odd")]
    #[test]
    fn as_columns(r: Region, n_cols: u32) {
        let regions = r.as_rows(n_cols);
        let w = regions[0].w;

        assert_eq!(regions.len(), n_cols as usize);
        assert!(regions.iter().all(|r| r.w == w));
    }

    #[test_case(0, 50, Some((50, 50)); "half width")]
    #[test_case(10, 50, Some((60, 40)); "offset half width")]
    #[test_case(0, 100, None; "at width")]
    #[test_case(0, 200, None; "out of range")]
    #[test]
    fn split_at_width(offset: u32, p: u32, expected: Option<(u32, u32)>) {
        let r = Region::new(offset, 0, 100, 100);
        let res = r.split_at_width(p + offset);

        if let Some((w1, w2)) = expected {
            assert!(res.is_ok());

            let (r1, r2) = res.unwrap();
            assert_eq!(r1, Region::new(offset, 0, w1, 100));
            assert_eq!(r2, Region::new(offset + w1, 0, w2, 100));
        } else {
            assert!(res.is_err());
        }
    }

    #[test_case(0, 50, Some((50, 50)); "half height")]
    #[test_case(10, 50, Some((60, 40)); "offset half height")]
    #[test_case(0, 100, None; "at height")]
    #[test_case(0, 200, None; "out of range")]
    #[test]
    fn split_at_height(offset: u32, p: u32, expected: Option<(u32, u32)>) {
        let r = Region::new(0, offset, 100, 100);
        let res = r.split_at_height(p + offset);

        if let Some((h1, h2)) = expected {
            assert!(res.is_ok());

            let (r1, r2) = res.unwrap();
            assert_eq!(r1, Region::new(0, offset, 100, h1));
            assert_eq!(r2, Region::new(0, offset + h1, 100, h2));
        } else {
            assert!(res.is_err());
        }
    }
}
