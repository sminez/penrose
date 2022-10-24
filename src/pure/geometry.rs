//! Geometry primitives
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

    /// Whether or not this [Point] is on the given [Line]
    pub fn on(&self, l: Line) -> bool {
        self.x >= l.a.x && self.x <= l.b.x && self.y >= l.a.y && self.y <= l.b.y
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

/// A directed line segment from `a` to `b`
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Line {
    /// The start of the line
    pub a: Point,
    /// The end of the line
    pub b: Point,
}

impl Line {
    /// A horizontal line from `a` extending `length` to the right
    pub fn horizontal<P>(a: P, length: u32) -> Self
    where
        P: Into<Point>,
    {
        let a = a.into();
        Self {
            a,
            b: Point {
                x: a.x + length,
                y: a.y,
            },
        }
    }

    /// A vertical line from `a` extending `length` down
    pub fn vertical<P>(a: P, length: u32) -> Self
    where
        P: Into<Point>,
    {
        let a = a.into();
        Self {
            a,
            b: Point {
                x: a.x,
                y: a.y + length,
            },
        }
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
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
        Rect { x, y, w, h }
    }

    /// The four corners of this [Rect] in [Point] form returned in clockwise
    /// order from the top left corener.
    pub fn corners(&self) -> (Point, Point, Point, Point) {
        let &Rect { x, y, w, h } = self;

        (
            Point { x, y },
            Point { x: x + w, y },
            Point { x: x + w, y: y + h },
            Point { x, y: y + h },
        )
    }

    /// The midpoint of this rectangle
    pub fn midpoint(&self) -> Point {
        Point {
            x: self.x + self.w / 2,
            y: self.y + self.h / 2,
        }
    }

    /// Create a new [Rect] with width equal to `factor` x `self.w`
    pub fn scale_w(&self, factor: f64) -> Self {
        Self {
            w: (self.w as f64 * factor).floor() as u32,
            ..*self
        }
    }

    /// Create a new [Rect] with height equal to `factor` x `self.h`
    pub fn scale_h(&self, factor: f64) -> Self {
        Self {
            h: (self.h as f64 * factor).floor() as u32,
            ..*self
        }
    }

    fn top_and_bottom(&self) -> (Line, Line) {
        let &Self { x, y, w, h } = self;

        (Line::horizontal((x, y), w), Line::horizontal((x, y + h), w))
    }

    fn left_and_right(&self) -> (Line, Line) {
        let &Self { x, y, w, h } = self;

        (Line::vertical((x, y), h), Line::vertical((x + w, y), h))
    }

    /// Check whether the given point is on the top or bottom edge of this [Rect].
    pub fn is_on_horizontal_edge<P>(&self, p: P) -> bool
    where
        P: Into<Point>,
    {
        let (top, bottom) = self.top_and_bottom();
        let p = p.into();

        p.on(top) || p.on(bottom)
    }

    /// Check whether the given point is on the left or right edge of this [Rect].
    pub fn is_on_vertical_edge<P>(&self, p: P) -> bool
    where
        P: Into<Point>,
    {
        let (left, right) = self.left_and_right();
        let p = p.into();

        p.on(left) || p.on(right)
    }

    /// Check whether the given point is on one of the edges of this [Rect].
    pub fn is_on_edge<P>(&self, p: P) -> bool
    where
        P: Into<Point>,
    {
        let p = p.into();

        self.is_on_horizontal_edge(p) || self.is_on_vertical_edge(p)
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
            .map(|n| Rect::new(self.x, (self.y + n as u32 * h) as u32, self.w, h))
            .collect()
    }

    /// Split this `Rect` into evenly sized columns.
    pub fn as_columns(&self, n_columns: u32) -> Vec<Rect> {
        if n_columns <= 1 {
            return vec![*self];
        }
        let w = self.w / n_columns as u32;
        (0..n_columns)
            .map(|n| Rect::new((self.x + n as u32 * w) as u32, self.y, w, self.h))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(1.5, Rect::new(10, 20, 45, 40); "scale up")]
    #[test_case(0.5, Rect::new(10, 20, 15, 40); "scale down")]
    #[test_case(1.0, Rect::new(10, 20, 30, 40); "unchanged")]
    #[test]
    fn scale_w(factor: f64, expected: Rect) {
        let r = Rect::new(10, 20, 30, 40);

        assert_eq!(r.scale_w(factor), expected);
    }

    #[test_case(1.5, Rect::new(10, 20, 30, 60); "scale up")]
    #[test_case(0.5, Rect::new(10, 20, 30, 20); "scale down")]
    #[test_case(1.0, Rect::new(10, 20, 30, 40); "unchanged")]
    #[test]
    fn scale_h(factor: f64, expected: Rect) {
        let r = Rect::new(10, 20, 30, 40);

        assert_eq!(r.scale_h(factor), expected);
    }

    #[test]
    fn contains_rect() {
        let r1 = Rect::new(10, 10, 50, 50);
        let r2 = Rect::new(0, 0, 100, 100);

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
        let r = Rect::new(10, 20, 30, 40);

        assert_eq!(r.contains_point(p), expected);
    }

    #[test_case(
        Rect::new(0, 0, 10, 10),
        Some(Rect::new(5, 5, 10, 10));
        "fits"
    )]
    #[test_case(
        Rect::new(10, 10, 10, 10),
        Some(Rect::new(5, 5, 10, 10));
        "fits overlaping"
    )]
    #[test_case(
        Rect::new(100, 100, 10, 10),
        Some(Rect::new(5, 5, 10, 10));
        "fits but not contained"
    )]
    #[test_case(Rect::new(0, 0, 100, 100), None; "doesn't fit")]
    #[test]
    fn centered_in(inner: Rect, expected: Option<Rect>) {
        let outer = Rect::new(0, 0, 20, 20);

        let res = inner.centered_in(&outer);

        assert_eq!(res, expected);
    }

    #[test_case(Rect::new(0, 0, 100, 100), 1; "simple single")]
    #[test_case(Rect::new(0, 0, 100, 100), 4; "simple even")]
    #[test_case(Rect::new(0, 0, 100, 100), 7; "simple odd")]
    #[test_case(Rect::new(0, 0, 79, 57), 1; "awkward single")]
    #[test_case(Rect::new(0, 0, 79, 57), 4; "awkward even")]
    #[test_case(Rect::new(0, 0, 79, 57), 7; "awkward odd")]
    #[test]
    fn as_rows(r: Rect, n_rows: u32) {
        let rects = r.as_rows(n_rows);
        let h = rects[0].h;

        assert_eq!(rects.len(), n_rows as usize);
        assert!(rects.iter().all(|r| r.h == h));
    }

    #[test_case(Rect::new(0, 0, 100, 100), 1; "simple single")]
    #[test_case(Rect::new(0, 0, 100, 100), 4; "simple even")]
    #[test_case(Rect::new(0, 0, 100, 100), 7; "simple odd")]
    #[test_case(Rect::new(0, 0, 79, 57), 1; "awkward single")]
    #[test_case(Rect::new(0, 0, 79, 57), 4; "awkward even")]
    #[test_case(Rect::new(0, 0, 79, 57), 7; "awkward odd")]
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
}
