//! Built-in layout transformers.
use crate::{
    core::layout::{Layout, LayoutTransformer},
    pure::geometry::Rect,
    simple_transformer, Xid,
};

simple_transformer!(
    /// Wrap an existing layout and reflect its window positions horizontally.
    ReflectHorizontal,
    reflect_horizontal,
    "Reflected"
);

fn reflect_horizontal(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    let mid = r.x + r.w / 2;

    positions
        .into_iter()
        .map(|(id, mut r)| {
            r.x = if r.x <= mid {
                2 * (mid - r.x) - r.w
            } else {
                2 * mid - r.x - r.w
            };

            (id, r)
        })
        .collect()
}

simple_transformer!(
    /// Wrap an existing layout and reflect its window positions vertically.
    ReflectVertical,
    reflect_vertical,
    "Flipped"
);

fn reflect_vertical(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    let mid = r.y + r.h / 2;

    positions
        .into_iter()
        .map(|(id, mut r)| {
            r.y = if r.y <= mid {
                2 * (mid - r.y) - r.h
            } else {
                2 * mid - r.y - r.h
            };

            (id, r)
        })
        .collect()
}

/// Simple gaps around the window placement of the enclosed [Layout].
///
/// `outer_px` controls the width of the gap around the edge of the screen and `inner_px`
/// controls the gap around each individual window. Set both equal to one another to have
/// a consistant gap size in all places.
#[derive(Debug, Clone)]
pub struct Gaps {
    /// The inner [Layout] having gaps applied to it.
    pub layout: Box<dyn Layout>,
    /// The desired outer gap size in pixels
    pub outer_px: u32,
    /// The desired inner gap size in pixels
    pub inner_px: u32,
}

impl Gaps {
    /// Wrap an existing [Layout] with the given gap sizes.
    pub fn wrap(layout: Box<dyn Layout>, outer_px: u32, inner_px: u32) -> Box<dyn Layout> {
        Box::new(Self {
            layout,
            outer_px,
            inner_px,
        })
    }
}

fn shrink(r: Rect, px: u32) -> Rect {
    if r.w == 0 || r.h == 0 {
        return r;
    }

    Rect {
        x: r.x + px,
        y: r.y + px,
        w: r.w - 2 * px,
        h: r.h - 2 * px,
    }
}

impl LayoutTransformer for Gaps {
    fn transformed_name(&self) -> String {
        self.layout.name()
    }

    fn inner_mut(&mut self) -> &mut Box<dyn Layout> {
        &mut self.layout
    }

    fn transform_initial(&self, r: Rect) -> Rect {
        shrink(r, self.outer_px)
    }

    fn transform_positions(&mut self, _: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
        positions
            .into_iter()
            .map(|(id, r)| (id, shrink(r, self.inner_px)))
            .collect()
    }
}

/// Reserve `px` pixels at the top of the screen.
///
/// Typically used for providing space for a status bar.
#[derive(Debug, Clone)]
pub struct ReserveTop {
    /// The wrapped inner layout
    pub layout: Box<dyn Layout>,
    /// The number of pixels to reserve at the top of the screen
    pub px: u32,
}

impl ReserveTop {
    /// Wrap an existing [Layout] with the given reserved area.
    pub fn wrap(layout: Box<dyn Layout>, px: u32) -> Box<dyn Layout> {
        Box::new(Self { layout, px })
    }
}

impl LayoutTransformer for ReserveTop {
    fn transformed_name(&self) -> String {
        self.layout.name()
    }

    fn inner_mut(&mut self) -> &mut Box<dyn Layout> {
        &mut self.layout
    }

    fn transform_initial(&self, mut r: Rect) -> Rect {
        if r.w == 0 || r.h == 0 {
            return r;
        }

        r.y += self.px;
        r.h -= self.px;

        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test_case(Rect::new(0, 0, 100, 200), Rect::new(0, 0, 100, 200); "fullscreen is idempotent")]
    #[test_case(Rect::new(0, 0, 40, 100), Rect::new(60, 0, 40, 100); "not crossing midpoint left")]
    #[test_case(Rect::new(60, 0, 40, 100), Rect::new(0, 0, 40, 100); "not crossing midpoint right")]
    #[test_case(Rect::new(0, 0, 60, 100), Rect::new(40, 0, 60, 100); "crossing midpoint")]
    #[test_case(Rect::new(0, 0, 50, 100), Rect::new(50, 0, 50, 100); "on midpoint")]
    #[test]
    fn reflect_horizontal(original: Rect, expected: Rect) {
        let r = Rect::new(0, 0, 100, 200);
        let transformed = reflect_horizontal(r, vec![(Xid(1), original)]);

        assert_eq!(transformed, vec![(Xid(1), expected)]);
    }

    #[test_case(Rect::new(0, 0, 100, 200), Rect::new(0, 0, 100, 200); "fullscreen is idempotent")]
    #[test_case(Rect::new(0, 0, 50, 80), Rect::new(0, 120, 50, 80); "not crossing midpoint above")]
    #[test_case(Rect::new(0, 120, 50, 80), Rect::new(0, 0, 50, 80); "not crossing midpoint below")]
    #[test_case(Rect::new(0, 0, 50, 120), Rect::new(0, 80, 50, 120); "crossing midpoint")]
    #[test_case(Rect::new(0, 0, 50, 100), Rect::new(0, 100, 50, 100); "on midpoint")]
    #[test]
    fn reflect_vertical(original: Rect, expected: Rect) {
        let r = Rect::new(0, 0, 100, 200);
        let transformed = reflect_vertical(r, vec![(Xid(1), original)]);

        assert_eq!(transformed, vec![(Xid(1), expected)]);
    }
}
