//! Built-in hooks
use crate::{
    core::{conn::Conn, hooks::LayoutHook, State, WinId},
    pure::geometry::Rect,
};

/// Simple gaps around the window placement of the enclosed [Layout][crate::core::layout::Layout].
///
/// `outer_px` controls the width of the gap around the edge of the screen and `inner_px`
/// controls the gap around each individual window. Set both equal to one another to have
/// a consistant gap size in all places.
#[derive(Debug, Clone, Default)]
pub struct SpacingHook {
    /// The desired outer gap size in pixels
    pub outer_px: u32,
    /// The desired inner gap size in pixels
    pub inner_px: u32,
    /// The number of pixels to reserve at the top of the screen
    pub top_px: u32,
    /// The number of pixels to reserve at the bottom of the screen
    pub bottom_px: u32,
}

impl<C: Conn> LayoutHook<C> for SpacingHook {
    fn transform_initial(&mut self, mut r: Rect, _: &State<C>, _: &mut C) -> Rect {
        if r.w == 0 || r.h == 0 {
            return r;
        }

        r.y += self.top_px as i32;
        r.h = r.h - self.top_px - self.bottom_px;

        shrink(r, self.outer_px)
    }

    fn transform_positions(
        &mut self,
        _: Rect,
        positions: Vec<(WinId, Rect)>,
        _: &State<C>,
        _: &mut C,
    ) -> Vec<(WinId, Rect)> {
        positions
            .into_iter()
            .map(|(id, r)| (id, shrink(r, self.inner_px)))
            .collect()
    }
}

fn shrink(r: Rect, px: u32) -> Rect {
    if r.w == 0 || r.h == 0 {
        return r;
    }

    Rect {
        x: r.x + px as i32,
        y: r.y + px as i32,
        w: r.w - 2 * px,
        h: r.h - 2 * px,
    }
}
