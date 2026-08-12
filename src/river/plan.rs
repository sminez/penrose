//! What the conn intends to tell river, and how it says it.
//!
//! River only accepts state changes inside a sequence it starts itself (see river-design.md §2),
//! which is nothing like penrose's "send it when the binding fires". So every mutating [Conn]
//! method writes into the plan here, and the plan is transmitted when a sequence arrives.
//!
//! Three properties the plan has to have, each learned the expensive way by other window
//! managers:
//!
//! - **It is a total restatement.** A sequence can start at any moment -- a new window needs no
//!   binding -- so the plan must always be complete and always safe to re-send. Penrose's refresh
//!   is partly diff based, so the conn accumulates those diffs and restates the whole thing.
//! - **One-shot effects are separate.** Re-sending a position is free; re-sending a close kills a
//!   second window. Those go in [Op], which drains rather than restating.
//! - **Liveness is filtered at transmit time.** The plan can name a window river has since
//!   closed, and every stale reference is a protocol error, so there is one guard in one place.
use crate::{
    Color,
    core::{
        bindings::{KeySym, MouseState},
        conn::WinId,
    },
    pure::geometry::Point,
    river::{
        Inner,
        protocol::river_window_management_v1::river_window_v1::{Capabilities, Edges},
    },
};
use std::collections::{HashMap, HashSet};
use tracing::trace;

/// State which changes what river says to a window: transmitted in a manage sequence.
#[derive(Debug, Default)]
pub(super) struct ManagePlan {
    /// Window content dimensions, from the `wh` half of `position_client`.
    pub(super) dimensions: HashMap<WinId, (u32, u32)>,
    /// The window to focus, or `None` for `clear_focus`.
    pub(super) focus: Option<WinId>,
    /// Windows which should be fullscreen.
    pub(super) fullscreen: HashSet<WinId>,
    /// Key bindings which should be live.
    pub(super) enabled: HashSet<KeySym>,
    /// Mouse bindings which should be live.
    pub(super) mouse_enabled: HashSet<MouseState>,
    /// Windows which have not yet been told how we decorate them.
    pub(super) initial_props: HashSet<WinId>,
    /// One-shot effects, drained rather than restated.
    pub(super) ops: Vec<Op>,
}

/// State which only changes what is drawn: transmitted in a render sequence.
#[derive(Debug, Default)]
pub(super) struct RenderPlan {
    /// Windows bottom to top, from `restack`.
    pub(super) order: Vec<WinId>,
    /// Window content positions, from the `xy` half of `position_client`.
    pub(super) positions: HashMap<WinId, Point>,
    /// Border colours. The width is the same for every window and lives on the conn.
    pub(super) borders: HashMap<WinId, Color>,
    /// Windows which are on a visible workspace.
    pub(super) visible: HashSet<WinId>,
}

/// An effect which must happen exactly once, rather than being restated every sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Op {
    /// Ask a window to close.
    Close(WinId),
    /// Warp the pointer to an absolute position in the compositor's coordinate space.
    WarpPointer(Point),
    /// Eat the next non-modifier key press.
    CaptureNextKey,
    /// Undo a capture which has not yet eaten anything.
    CancelCaptureNextKey,
}

impl Inner {
    /// Transmit the manage half of the plan and finish the sequence.
    pub(super) fn transmit_manage(&mut self) {
        trace!("transmitting manage plan");

        // Ops first: a close in the same sequence as a position for the same window is
        // pointless work, but harmless, and draining first keeps the ops from being lost if
        // anything below decides to bail out.
        for op in std::mem::take(&mut self.manage.ops) {
            self.transmit_op(op);
        }

        for (&id, &(w, h)) in self.manage.dimensions.iter() {
            if let Some(win) = self.live_window(id) {
                win.propose_dimensions(w as i32, h as i32);
            }
        }

        for &id in self.manage.initial_props.iter() {
            if let Some(win) = self.live_window(id) {
                // River draws the borders, so the window should not: penrose has no CSD support
                // and would not know how big the client's own decorations were.
                win.use_ssd();
                win.set_tiled(Edges::all());
                // Penrose honours neither maximize nor minimize, and fullscreen is driven by the
                // window manager rather than by the window, so windows are told none of it.
                win.set_capabilities(Capabilities::empty());
            }
        }
        self.manage.initial_props.clear();

        for (&id, win) in self.windows.iter() {
            let Some(obj) = win.obj.as_ref() else {
                continue;
            };
            let wants_fullscreen = self.manage.fullscreen.contains(&id);

            if wants_fullscreen == win.fullscreen_set {
                continue;
            }

            match self.output_for(id) {
                // River needs to be told which output to fill, where X11 fullscreens a window
                // where it already is. The output the window is laid out on is the same thing.
                Some(output) if wants_fullscreen => obj.fullscreen(output),
                None if wants_fullscreen => (),
                _ => obj.exit_fullscreen(),
            }
        }
        let fullscreen = self.manage.fullscreen.clone();
        for (id, win) in self.windows.iter_mut() {
            win.fullscreen_set = fullscreen.contains(id);
        }

        // While a layer surface holds keyboard focus exclusively -- a lock screen, or a menu --
        // river ignores everything we say about focus, so there is nothing to say. The plan
        // restates focus every sequence, so it is re-asserted when the surface goes away.
        if !self.focus_is_exclusive {
            match self
                .manage
                .focus
                .and_then(|id| self.live_window(id).cloned())
            {
                Some(win) => self.for_each_seat(|s| s.obj.focus_window(&win)),
                None => self.for_each_seat(|s| s.obj.clear_focus()),
            }
        }

        self.transmit_bindings();

        if let Some(ls) = self.default_layer_output() {
            ls.set_default();
        }

        self.plan_dirty = false;
        self.dirty_requested = false;
        self.wm.manage_finish();
    }

    /// Transmit the render half of the plan and finish the sequence.
    pub(super) fn transmit_render(&mut self) {
        trace!("transmitting render plan");

        for (&id, &p) in self.render.positions.iter() {
            if let Some(node) = self.live_node(id) {
                node.set_position(p.x, p.y);
            }
        }

        let width = self.border_width as i32;
        for (&id, &color) in self.render.borders.iter() {
            if let Some(win) = self.live_window(id) {
                let (r, g, b, a) = scale_color(color);
                win.set_borders(Edges::all(), width, r, g, b, a);
            }
        }

        for (&id, win) in self.windows.iter() {
            let Some(obj) = win.obj.as_ref() else {
                continue;
            };

            // Everything penrose has not placed on a visible workspace is hidden, which is what
            // makes river need no workspace concept of its own.
            if self.render.visible.contains(&id) {
                obj.show();
            } else {
                obj.hide();
            }
        }

        // Each node above the last, which is the same bottom to top ordering restack takes.
        let mut previous: Option<_> = None;
        for id in self.render.order.clone() {
            let Some(node) = self.live_node(id).cloned() else {
                continue;
            };

            if let Some(below) = previous {
                node.place_above(&below);
            }
            previous = Some(node);
        }

        self.wm.render_finish();

        // A render sequence does not carry manage state, so anything that arrived while this one
        // was open still needs a sequence of its own.
        self.request_manage_sequence();
    }

    fn transmit_op(&mut self, op: Op) {
        match op {
            Op::Close(id) => {
                if let Some(win) = self.live_window(id) {
                    win.close();
                }
            }

            Op::WarpPointer(p) => self.for_each_seat(|s| s.obj.pointer_warp(p.x, p.y)),

            Op::CaptureNextKey => self.for_each_seat(|s| {
                if let Some(xkb) = s.xkb.as_ref() {
                    xkb.ensure_next_key_eaten();
                }
            }),

            Op::CancelCaptureNextKey => self.for_each_seat(|s| {
                if let Some(xkb) = s.xkb.as_ref() {
                    xkb.cancel_ensure_next_key_eaten();
                }
            }),
        }
    }
}

/// River's border colour channels are 32 bit and read as a percentage, where penrose's are bytes.
fn scale_color(color: Color) -> (u32, u32, u32, u32) {
    let scale = |n: u32| (n & 0xff) * 0x01010101;
    let hex = color.rgba_u32();

    (
        scale(hex >> 24),
        scale(hex >> 16),
        scale(hex >> 8),
        scale(hex),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_scale_to_full_range() {
        // 0x00 and 0xff have to land exactly on the ends of river's range, or a black border is
        // slightly transparent and a white one is slightly grey.
        let (r, g, b, a) = scale_color(Color::new_from_hex(0xff0000ff));

        assert_eq!(r, 0xffffffff);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
        assert_eq!(a, 0xffffffff);
    }

    #[test]
    fn color_channels_are_in_rgba_order() {
        let (r, g, b, a) = scale_color(Color::new_from_hex(0x11223344));

        assert_eq!(
            (r, g, b, a),
            (0x11111111, 0x22222222, 0x33333333, 0x44444444)
        );
    }
}
