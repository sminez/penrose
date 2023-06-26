//! Geometry primitives
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::cmp::max;

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

impl From<(u32, u32)> for Point {
    fn from(raw: (u32, u32)) -> Self {
        let (x, y) = raw;

        Self { x, y }
    }
}

impl From<(&u32, &u32)> for Point {
    fn from(raw: (&u32, &u32)) -> Self {
        let (&x, &y) = raw;

        Self { x, y }
    }
}

// A Rect converts to its top left corner
impl From<Rect> for Point {
    fn from(r: Rect) -> Self {
        let Rect { x, y, .. } = r;

        Self { x, y }
    }
}

impl From<&Rect> for Point {
    fn from(r: &Rect) -> Self {
        let &Rect { x, y, .. } = r;

        Self { x, y }
    }
}

/// An X window / screen position: top left corner + extent as percentages
/// of the current screen containing the window.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct RelativeRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl RelativeRect {
    /// Create a new RelativeRect from the provided values.
    ///
    /// Values are clamped to be in the range 0.0 to 1.0.
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            w: w.clamp(0.0, 1.0),
            h: h.clamp(0.0, 1.0),
        }
    }

    /// All available space within a given Rect
    pub fn fullscreen() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        }
    }

    /// Apply the proportions of this RelativeRect to a given Rect.
    pub fn applied_to(&self, r: &Rect) -> Rect {
        Rect {
            x: r.x + (r.w as f64 * self.x).floor() as u32,
            y: r.y + (r.h as f64 * self.y).floor() as u32,
            w: (r.w as f64 * self.w).floor() as u32,
            h: (r.h as f64 * self.h).floor() as u32,
        }
    }

    /// Apply some [Rect] based operation to this [RelativeRect] by applying it
    /// to a given reference [Rect].
    pub fn apply_as_rect<F>(self, r: &Rect, f: F) -> Self
    where
        F: Fn(Rect) -> Rect,
    {
        f(self.applied_to(r)).relative_to(r)
    }
}

/// Something that can be converted into a [RelativeRect] by comparing to
/// some reference [Rect].
pub trait RelativeTo {
    /// Convert to a [RelativeRect] using the reference [Rect]
    fn relative_to(&self, r: &Rect) -> RelativeRect;
}

impl RelativeTo for RelativeRect {
    fn relative_to(&self, _r: &Rect) -> RelativeRect {
        *self
    }
}

// TODO: the current implemention will produce essentially garbage results if the
//       child Rect is not a subregion of the parent. This needs bounds checking
//       and some sensible default behaviour when those checks fail (such as translating
//       the Rect to fit or scaling it down)
impl RelativeTo for Rect {
    fn relative_to(&self, r: &Rect) -> RelativeRect {
        RelativeRect::new(
            (self.x - r.x) as f64 / r.w as f64,
            (self.y - r.y) as f64 / r.h as f64,
            self.w as f64 / r.w as f64,
            self.h as f64 / r.h as f64,
        )
    }
}

/// An X window / screen position: top left corner + extent
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Rect {
    /// The x-coordinate of the top left corner of this rect
    pub x: u32,
    /// The y-coordinate of the top left corner of this rect
    pub y: u32,
    /// The width of this rect
    pub w: u32,
    /// The height of this rect
    pub h: u32,
}

impl Rect {
    /// Create a new Rect.
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The four corners of this [Rect] in [Point] form returned in clockwise
    /// order from the top left corner.
    /// ```
    /// # use penrose::pure::geometry::{Rect, Point};
    /// let r = Rect::new(0, 0, 100, 200);
    /// let corners = r.corners();
    ///
    /// assert_eq!(
    ///     corners,
    ///     (
    ///         Point { x: 0, y: 0 },
    ///         Point { x: 100, y: 0 },
    ///         Point { x: 100, y: 200 },
    ///         Point { x: 0, y: 200 },
    ///     )
    /// );
    /// ```
    pub fn corners(&self) -> (Point, Point, Point, Point) {
        let &Rect { x, y, w, h } = self;

        (
            Point { x, y },
            Point { x: x + w, y },
            Point { x: x + w, y: y + h },
            Point { x, y: y + h },
        )
    }

    /// The midpoint of this rectangle.
    ///
    /// Odd side lengths will lead to a truncated point towards the top left corner
    /// in order to maintain integer coordinates.
    /// ```
    /// # use penrose::pure::geometry::{Rect, Point};
    /// let r = Rect::new(0, 0, 100, 200);
    ///
    /// assert_eq!(r.midpoint(), Point { x: 50, y: 100 });
    /// ```
    pub fn midpoint(&self) -> Point {
        Point {
            x: self.x + self.w / 2,
            y: self.y + self.h / 2,
        }
    }

    /// Shrink width and height by the given pixel border, maintaining the current x and y
    /// coordinates. The resulting `Rect` will always have a minimum width and height of 1.
    /// ```
    /// # use penrose::pure::geometry::Rect;
    /// let r = Rect::new(0, 0, 100, 200);
    ///
    /// assert_eq!(r.shrink_in(10), Rect::new(0, 0, 80, 180));
    /// assert_eq!(r.shrink_in(50), Rect::new(0, 0, 1, 100));
    /// assert_eq!(r.shrink_in(100), Rect::new(0, 0, 1, 1));
    /// ```
    pub fn shrink_in(&self, border: u32) -> Self {
        let w = if self.w <= 2 * border {
            1
        } else {
            self.w - 2 * border
        };
        let h = if self.h <= 2 * border {
            1
        } else {
            self.h - 2 * border
        };

        Self { w, h, ..*self }
    }

    /// Create a new [Rect] with width equal to `factor` x `self.w`
    /// ```
    /// # use penrose::pure::geometry::Rect;
    /// let r = Rect::new(0, 0, 30, 40);
    ///
    /// assert_eq!(r.scale_w(1.5), Rect::new(0, 0, 45, 40));
    /// assert_eq!(r.scale_w(0.5), Rect::new(0, 0, 15, 40));
    /// ```
    pub fn scale_w(&self, factor: f64) -> Self {
        Self {
            w: (self.w as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Create a new [Rect] with height equal to `factor` x `self.h`
    /// ```
    /// # use penrose::pure::geometry::Rect;
    /// let r = Rect::new(0, 0, 30, 40);
    ///
    /// assert_eq!(r.scale_h(1.5), Rect::new(0, 0, 30, 60));
    /// assert_eq!(r.scale_h(0.5), Rect::new(0, 0, 30, 20));
    /// ```
    pub fn scale_h(&self, factor: f64) -> Self {
        Self {
            h: (self.h as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Update the width and height of this [Rect] by specified deltas.
    ///
    /// Minimum size is clamped at 1x1.
    ///
    /// # Panics
    /// This function will panic if one of the supplied deltas overflows `i32::MAX`.
    /// ```
    /// # use penrose::pure::geometry::Rect;
    /// let mut r = Rect::new(0, 0, 100, 200);
    ///
    /// r.resize(20, 30);
    /// assert_eq!(r, Rect::new(0, 0, 120, 230));
    ///
    /// r.resize(-40, -50);
    /// assert_eq!(r, Rect::new(0, 0, 80, 180));
    /// ```
    pub fn resize(&mut self, dw: i32, dh: i32) {
        self.w = max(1, (self.w as i32) + dw) as u32;
        self.h = max(1, (self.h as i32) + dh) as u32;
    }

    /// Update the position of this [Rect] by specified deltas.
    ///
    /// Minimum (x, y) coordinates are clamped at (0, 0)
    ///
    /// # Panics
    /// This function will panic if one of the supplied deltas overflows `i32::MAX`.
    /// ```
    /// # use penrose::pure::geometry::Rect;
    /// let mut r = Rect::new(0, 0, 100, 200);
    ///
    /// r.reposition(20, 30);
    /// assert_eq!(r, Rect::new(20, 30, 100, 200));
    ///
    /// r.reposition(-40, -20);
    /// assert_eq!(r, Rect::new(0, 10, 100, 200));
    /// ```
    pub fn reposition(&mut self, dx: i32, dy: i32) {
        self.x = max(0, (self.x as i32) + dx) as u32;
        self.y = max(0, (self.y as i32) + dy) as u32;
    }

    /// Check whether this Rect contains `other` as a sub-Rect
    pub fn contains(&self, other: &Rect) -> bool {
        match other {
            Rect { x, .. } if *x < self.x => false,
            Rect { x, w, .. } if (*x + *w) > (self.x + self.w) => false,
            Rect { y, .. } if *y < self.y => false,
            Rect { y, h, .. } if (*y + *h) > (self.y + self.h) => false,
            _ => true,
        }
    }

    /// Check whether this Rect is physically larger than `other` regardless
    /// of position.
    pub fn is_larger_than(&self, other: &Rect) -> bool {
        self.w > other.w && self.h > other.h
    }

    /// Check whether this Rect contains `p`
    pub fn contains_point<P>(&self, p: P) -> bool
    where
        P: Into<Point>,
    {
        let p = p.into();

        (self.x..(self.x + self.w + 1)).contains(&p.x)
            && (self.y..(self.y + self.h + 1)).contains(&p.y)
    }

    /// Center this Rect inside of `enclosing`.
    ///
    /// Returns `None` if this Rect can not fit inside enclosing
    pub fn centered_in(&self, enclosing: &Rect) -> Option<Self> {
        if self.w > enclosing.w || self.h > enclosing.h {
            return None;
        }

        Some(Self {
            x: enclosing.x + ((enclosing.w - self.w) / 2),
            y: enclosing.y + ((enclosing.h - self.h) / 2),
            ..*self
        })
    }

    /// Split this `Rect` into evenly sized rows.
    pub fn as_rows(&self, n_rows: u32) -> Vec<Rect> {
        if n_rows <= 1 {
            return vec![*self];
        }
        let h = self.h / n_rows;
        (0..n_rows)
            .map(|n| Rect::new(self.x, self.y + n * h, self.w, h))
            .collect()
    }

    /// Split this `Rect` into evenly sized columns.
    pub fn as_columns(&self, n_columns: u32) -> Vec<Rect> {
        if n_columns <= 1 {
            return vec![*self];
        }
        let w = self.w / n_columns;
        (0..n_columns)
            .map(|n| Rect::new(self.x + n * w, self.y, w, self.h))
            .collect()
    }

    /// Divides this rect into two columns where the first has the given width.
    ///
    /// Returns `None` if new_width is out of bounds
    pub fn split_at_width(&self, new_width: u32) -> Option<(Self, Self)> {
        if new_width >= self.w {
            None
        } else {
            Some((
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

    /// Divides this rect into two rows where the first has the given height.
    ///
    /// Returns `None` if new_height is out of bounds
    pub fn split_at_height(&self, new_height: u32) -> Option<(Self, Self)> {
        if new_height >= self.h {
            None
        } else {
            Some((
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

    /// Divides this rect into two columns along its midpoint.
    pub fn split_at_mid_width(&self) -> (Self, Self) {
        let new_width = self.w / 2;
        (
            Self {
                w: new_width,
                ..*self
            },
            Self {
                x: self.x + new_width,
                w: self.w - new_width,
                ..*self
            },
        )
    }

    /// Divides this rect into two rows along its midpoint.
    pub fn split_at_mid_height(&self) -> (Self, Self) {
        let new_height = self.h / 2;
        (
            Self {
                h: new_height,
                ..*self
            },
            Self {
                y: self.y + new_height,
                h: self.h - new_height,
                ..*self
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    // Helpers to make it easier to read the cases for the tests below

    fn r(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect::new(x, y, w, h)
    }

    fn rr(x: f64, y: f64, w: f64, h: f64) -> RelativeRect {
        RelativeRect::new(x, y, w, h)
    }

    fn p(x: u32, y: u32) -> Point {
        Point { x, y }
    }

    #[test]
    fn corners_works() {
        let r = Rect::new(0, 0, 1, 2);
        let corners = r.corners();

        assert_eq!(
            corners,
            (
                Point { x: 0, y: 0 },
                Point { x: 1, y: 0 },
                Point { x: 1, y: 2 },
                Point { x: 0, y: 2 },
            )
        );
    }

    #[test_case(r(0, 0, 10, 20), p(5, 10); "even both")]
    #[test_case(r(0, 0, 10, 21), p(5, 10); "even width")]
    #[test_case(r(0, 0, 11, 20), p(5, 10); "even height")]
    #[test_case(r(0, 0, 11, 21), p(5, 10); "odd both")]
    #[test]
    fn midpoint_works(r: Rect, p: Point) {
        assert_eq!(r.midpoint(), p);
    }

    #[test_case(r(0, 0, 10, 20), 1, 8, 18; "small border")]
    #[test_case(r(0, 0, 10, 20), 1000, 1, 1; "massive border")]
    #[test_case(r(0, 0, 10, 20), 5, 1, 10; "border half of width")]
    #[test_case(r(0, 0, 20, 10), 5, 10, 1; "border half of height")]
    #[test]
    fn shrink_in_works(r: Rect, b: u32, w: u32, h: u32) {
        let res = r.shrink_in(b);
        assert_eq!(
            res,
            Rect {
                x: r.x,
                y: r.y,
                w,
                h
            }
        )
    }

    #[test_case(1.5, r(10, 20, 45, 40); "scale up")]
    #[test_case(0.5, r(10, 20, 15, 40); "scale down")]
    #[test_case(1.0, r(10, 20, 30, 40); "unchanged")]
    #[test]
    fn scale_w(factor: f64, expected: Rect) {
        let r = Rect::new(10, 20, 30, 40);

        assert_eq!(r.scale_w(factor), expected);
    }

    #[test_case(1.5, r(10, 20, 30, 60); "scale up")]
    #[test_case(0.5, r(10, 20, 30, 20); "scale down")]
    #[test_case(1.0, r(10, 20, 30, 40); "unchanged")]
    #[test]
    fn scale_h(factor: f64, expected: Rect) {
        let r = Rect::new(10, 20, 30, 40);

        assert_eq!(r.scale_h(factor), expected);
    }

    // no case for increase by i32::MAX as this overflows (documented).
    #[test_case(1, 2, r(0, 0, 11, 22); "increase")]
    #[test_case(-1, -2, r(0, 0, 9, 18); "decrease")]
    #[test_case(-100, -100, r(0, 0, 1, 1); "clamp at 1x1")]
    #[test_case(-i32::MAX, -i32::MAX, r(0, 0, 1, 1); "decrease by max")]
    #[test]
    fn resize_works(dw: i32, dh: i32, expected: Rect) {
        let mut r = Rect::new(0, 0, 10, 20);
        r.resize(dw, dh);

        assert_eq!(r, expected);
    }

    // no case for increase by i32::MAX as this overflows (documented).
    #[test_case(1, 2, r(11, 22, 10, 20); "increase")]
    #[test_case(-1, -2, r(9, 18, 10, 20); "decrease")]
    #[test_case(-100, -100, r(0, 0, 10, 20); "clamp at 0x0")]
    #[test_case(-i32::MAX, -i32::MAX, r(0, 0, 10, 20); "decrease by max")]
    #[test]
    fn reposition_works(dw: i32, dh: i32, expected: Rect) {
        let mut r = Rect::new(10, 20, 10, 20);
        r.reposition(dw, dh);

        assert_eq!(r, expected);
    }

    #[test]
    fn contains_rect() {
        let r1 = Rect::new(10, 10, 50, 50);
        let r2 = Rect::new(0, 0, 100, 100);

        assert!(r2.contains(&r1));
        assert!(!r1.contains(&r2));
    }

    #[test_case(p(0, 0), false; "outside")]
    #[test_case(p(30, 20), true; "inside")]
    #[test_case(p(10, 20), true; "top left")]
    #[test_case(p(40, 20), true; "top right")]
    #[test_case(p(10, 60), true; "bottom left")]
    #[test_case(p(40, 60), true; "bottom right")]
    #[test]
    fn contains_point(p: Point, expected: bool) {
        let r = Rect::new(10, 20, 30, 40);

        assert_eq!(r.contains_point(p), expected);
    }

    #[test_case(r(0, 0, 10, 10), Some(r(5, 5, 10, 10)); "fits")]
    #[test_case(r(10, 10, 10, 10), Some(r(5, 5, 10, 10)); "fits overlaping")]
    #[test_case(r(100, 100, 10, 10), Some(r(5, 5, 10, 10)); "fits but not contained")]
    #[test_case(r(0, 0, 100, 100), None; "doesn't fit")]
    #[test]
    fn centered_in(inner: Rect, expected: Option<Rect>) {
        let outer = Rect::new(0, 0, 20, 20);

        let res = inner.centered_in(&outer);

        assert_eq!(res, expected);
    }

    #[test_case(r(0, 0, 100, 100), 1; "simple single")]
    #[test_case(r(0, 0, 100, 100), 4; "simple even")]
    #[test_case(r(0, 0, 100, 100), 7; "simple odd")]
    #[test_case(r(0, 0, 79, 57), 1; "awkward single")]
    #[test_case(r(0, 0, 79, 57), 4; "awkward even")]
    #[test_case(r(0, 0, 79, 57), 7; "awkward odd")]
    #[test]
    fn as_rows(r: Rect, n_rows: u32) {
        let rects = r.as_rows(n_rows);
        let h = rects[0].h;

        assert_eq!(rects.len(), n_rows as usize);
        assert!(rects.iter().all(|r| r.h == h));
    }

    #[test_case(r(0, 0, 100, 100), 1; "simple single")]
    #[test_case(r(0, 0, 100, 100), 4; "simple even")]
    #[test_case(r(0, 0, 100, 100), 7; "simple odd")]
    #[test_case(r(0, 0, 79, 57), 1; "awkward single")]
    #[test_case(r(0, 0, 79, 57), 4; "awkward even")]
    #[test_case(r(0, 0, 79, 57), 7; "awkward odd")]
    #[test]
    fn as_columns(r: Rect, n_cols: u32) {
        let rects = r.as_rows(n_cols);
        let w = rects[0].w;

        assert_eq!(rects.len(), n_cols as usize);
        assert!(rects.iter().all(|r| r.w == w));
    }

    #[test_case(0, 50, Some((50, 50)); "half width")]
    #[test_case(10, 50, Some((60, 40)); "offset half width")]
    #[test_case(0, 100, None; "at width")]
    #[test_case(0, 200, None; "out of range")]
    #[test]
    fn split_at_width(offset: u32, p: u32, expected: Option<(u32, u32)>) {
        let r = Rect::new(offset, 0, 100, 100);
        let res = r.split_at_width(p + offset);

        if let Some((w1, w2)) = expected {
            assert!(res.is_some());

            let (r1, r2) = res.unwrap();
            assert_eq!(r1, Rect::new(offset, 0, w1, 100));
            assert_eq!(r2, Rect::new(offset + w1, 0, w2, 100));
        } else {
            assert!(res.is_none());
        }
    }

    #[test_case(0, 50, Some((50, 50)); "half height")]
    #[test_case(10, 50, Some((60, 40)); "offset half height")]
    #[test_case(0, 100, None; "at height")]
    #[test_case(0, 200, None; "out of range")]
    #[test]
    fn split_at_height(offset: u32, p: u32, expected: Option<(u32, u32)>) {
        let r = Rect::new(0, offset, 100, 100);
        let res = r.split_at_height(p + offset);

        if let Some((h1, h2)) = expected {
            assert!(res.is_some());

            let (r1, r2) = res.unwrap();
            assert_eq!(r1, Rect::new(0, offset, 100, h1));
            assert_eq!(r2, Rect::new(0, offset + h1, 100, h2));
        } else {
            assert!(res.is_none());
        }
    }

    #[test_case(r(0, 0, 200, 100), r(0, 0, 200, 100), rr(0.0, 0.0, 1.0, 1.0); "fullscreen")]
    #[test_case(r(0, 0, 50, 50), r(0, 0, 200, 100), rr(0.0, 0.0, 0.25, 0.5); "subregion with same xy")]
    #[test_case(r(10, 10, 50, 50), r(0, 0, 200, 100), rr(0.05, 0.1, 0.25, 0.5); "subregion with different xy")]
    #[test_case(r(110, 110, 50, 50), r(100, 100, 200, 100), rr(0.05, 0.1, 0.25, 0.5); "subregion with different xy and parent not at origin")]
    #[test]
    fn relative_to_rect(child: Rect, parent: Rect, expected: RelativeRect) {
        let relative = child.relative_to(&parent);

        assert_eq!(relative, expected);
    }

    #[test]
    fn apply_as_rect_resize() {
        let relative = rr(0.0, 0.0, 0.8, 0.8);
        let reference = r(0, 0, 2000, 1000);

        let res = relative.apply_as_rect(&reference, |mut r| {
            r.resize(-10, 0);
            r
        });

        assert_eq!(res, rr(0.0, 0.0, 0.795, 0.8));
    }

    #[test]
    fn apply_as_rect_reposition() {
        let relative = rr(0.0, 0.0, 0.8, 0.8);
        let reference = r(0, 0, 2000, 1000);

        let res = relative.apply_as_rect(&reference, |mut r| {
            r.reposition(10, 0);
            r
        });

        assert_eq!(res, rr(0.005, 0.0, 0.8, 0.8));
    }
}
