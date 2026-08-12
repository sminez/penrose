//! What the conn intends to tell river, and how it says it.
//!
//! River only accepts state changes inside a sequence it starts itself (see river-design.md §2),
//! which is nothing like penrose's "send it when the binding fires". So every mutating [Conn]
//! method writes into the plan here, the worker publishes it, and the loop transmits it when a
//! sequence arrives.
//!
//! Three properties the plan has to have, each learned the expensive way by other window
//! managers:
//!
//! - **It is a total restatement.** A sequence can start at any moment -- a new window needs no
//!   binding -- so the plan must always be complete and always safe to re-send. Penrose's refresh
//!   is partly diff based, so the conn accumulates those diffs and restates the whole thing. It
//!   is also what lets the loop answer a sequence the worker has not caught up with: re-affirming
//!   the plan it already has is always a valid sequence.
//! - **One-shot effects are separate.** Re-sending a position is free; re-sending a close kills a
//!   second window. Those go in [Op], which drains rather than restating.
//! - **Liveness is filtered at transmit time.** The worker's view is always slightly stale, so a
//!   plan can name a window river has since closed, and every stale reference is a protocol
//!   error. One guard, in one place, on the loop.
//!
//! [Conn]: crate::core::conn::Conn
use crate::{
    Color,
    core::{
        bindings::{KeySym, MouseState},
        conn::WinId,
    },
    pure::geometry::Point,
    river::{
        protocol::river_window_management_v1::river_window_v1::{Capabilities, Edges},
        wayland::Loop,
    },
};
use std::collections::{HashMap, HashSet};
use tracing::trace;

/// State which changes what river says to a window: transmitted in a manage sequence.
#[derive(Debug, Default, Clone)]
pub(super) struct ManagePlan {
    /// Window content dimensions, from the `wh` half of `position_client`.
    pub(super) dimensions: HashMap<WinId, (u32, u32)>,
    /// The window to focus, or `None` for `clear_focus`.
    pub(super) focus: Option<WinId>,
    /// Windows which should be fullscreen.
    pub(super) fullscreen: HashSet<WinId>,
    /// Windows which have been told how we decorate them. Restated rather than drained: the
    /// requests are idempotent, and the loop may be transmitting a plan the worker has moved on
    /// from.
    pub(super) initial_props: HashSet<WinId>,
}

/// State which only changes what is drawn: transmitted in a render sequence.
#[derive(Debug, Default, Clone)]
pub(super) struct RenderPlan {
    /// Windows bottom to top, from `restack`.
    pub(super) order: Vec<WinId>,
    /// Window content positions, from the `xy` half of `position_client`.
    pub(super) positions: HashMap<WinId, Point>,
    /// Border colours, and widths: the width is per window rather than global, because penrose
    /// asks for no border on a window filling its screen.
    pub(super) borders: HashMap<WinId, Color>,
    pub(super) border_widths: HashMap<WinId, u32>,
    /// Windows which are on a visible workspace.
    pub(super) visible: HashSet<WinId>,
}

/// An effect which must happen exactly once, rather than being restated every sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Op {
    /// Ask a window to close.
    Close(WinId),
    /// Warp the pointer to an absolute position in the compositor's coordinate space.
    WarpPointer(Point),
    /// Replace the set of bindings river should match against.
    Grab {
        keys: Vec<KeySym>,
        mouse: Vec<MouseState>,
    },
    /// Eat the next non-modifier key press, listening for the keys which would continue the
    /// sequence in progress.
    Capture(Vec<KeySym>),
    /// Undo a capture which has not yet eaten anything.
    CancelCapture,
}

impl Loop {
    /// Transmit the manage half of the plan and finish the sequence.
    pub(super) fn transmit_manage(&mut self, ops: Vec<Op>) {
        trace!("transmitting manage plan");

        // Ops first, so that the input routing they change is what the rest of the sequence
        // enables, and so that a close is not preceded by a pointless resize of the same window.
        for op in ops {
            self.transmit_op(op);
        }

        for (&id, &(w, h)) in self.manage.dimensions.iter() {
            if let Some(win) = self.live_window(id) {
                trace!(%id, w, h, "proposing dimensions");
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

        self.wm.manage_finish();
    }

    /// Transmit the render half of the plan and finish the sequence.
    pub(super) fn transmit_render(&mut self) {
        trace!("transmitting render plan");

        for (&id, &p) in self.render.positions.iter() {
            if let Some(node) = self.live_node(id) {
                // The other half of what a layout change sends. Paired with the
                // proposed dimensions in the manage sequence, this is the whole
                // of what a window was told, which is the first thing wanted
                // when one ends up the wrong size or in the wrong place.
                trace!(%id, x = p.x, y = p.y, "positioning");
                node.set_position(p.x, p.y);
            }
        }

        for (&id, &color) in self.render.borders.iter() {
            if let Some(win) = self.live_window(id) {
                // A width of zero disables the border, which is what penrose asks for when a
                // window fills its screen.
                let width = self.render.border_widths.get(&id).copied().unwrap_or(0) as i32;
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
    }

    fn transmit_op(&mut self, op: Op) {
        match op {
            Op::Close(id) => {
                if let Some(win) = self.live_window(id) {
                    win.close();
                }
            }

            Op::WarpPointer(p) => self.for_each_seat(|s| s.obj.pointer_warp(p.x, p.y)),

            Op::Grab { keys, mouse } => self.grab(keys, mouse),
            Op::Capture(continuations) => self.capture(continuations),
            Op::CancelCapture => self.cancel_capture(),
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
