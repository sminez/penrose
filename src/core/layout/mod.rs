//! Layout for window positioning
use crate::{
    core::Xid,
    geometry::Rect,
    state::{Stack, Workspace},
};

pub mod messages;

use messages::{common::*, Message};

// TODO: do I also need versions of these with access to state?
pub trait Layout {
    fn name(&self) -> String;

    fn layout_workspace(
        self: Box<Self>,
        w: &Workspace<Xid>,
        r: Rect,
    ) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        match &w.stack {
            Some(s) => self.layout(s, r),
            None => self.layout_empty(r),
        }
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>);

    fn layout_empty(self: Box<Self>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>);

    fn handle_message(&mut self, m: &Message);
}

/// A stack of [Layout] options for use on a particular [Workspace].
///
/// The [Stack] itself acts as a [Layout], deferring all operations to the
/// currently focused Layout.
pub type LayoutStack = Stack<Box<dyn Layout>>;

impl LayoutStack {
    // NOTE: We allow for swapping out the current layout for a new one when layout operations
    // run so we can't just deref down to the focus directly.
    fn run_and_replace<F>(self, f: F) -> (Box<dyn Layout>, Vec<(Xid, Rect)>)
    where
        F: FnOnce(Box<dyn Layout>) -> (Box<dyn Layout>, Vec<(Xid, Rect)>),
    {
        let Self { up, focus, down } = self;
        let (new_focus, rs) = (f)(focus);
        let new = Box::new(Stack {
            up,
            focus: new_focus,
            down,
        });

        (new, rs)
    }

    /// Send the given [Message] to every [Layout] in this stack rather that just the
    /// currently focused one.
    pub fn broadcast_message(&mut self, m: &Message) {
        self.iter_mut().for_each(|l| l.handle_message(m))
    }
}

impl Layout for LayoutStack {
    fn name(&self) -> String {
        self.focus.name()
    }

    fn layout_workspace(
        self: Box<Self>,
        w: &Workspace<Xid>,
        r: Rect,
    ) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        self.run_and_replace(|l| l.layout_workspace(w, r))
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        self.run_and_replace(|l| l.layout(s, r))
    }

    fn layout_empty(self: Box<Self>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        self.run_and_replace(|l| l.layout_empty(r))
    }

    fn handle_message(&mut self, m: &Message) {
        self.focus.handle_message(m)
    }
}

#[derive(Debug, Clone, Copy)]
enum StackPosition {
    Side,
    Bottom,
}

/// A simple [Layout] with main and secondary regions.
///
/// - `MainAndStack::side` give a main region to the left and remaining clients to the right.
/// - `MainAndStack::bottom` give a main region to the top and remaining clients to the bottom.
///
/// The ratio between the main and secondary stack regions can be adjusted by sending [ShrinkMain]
/// and [ExpandMain] messages to this layout. The number of clients in the main area can be
/// increased or decreased by sending an [IncMain] message. To flip between the side and bottom
/// behaviours you can send a [Rotate] message.
#[derive(Debug, Clone, Copy)]
pub struct MainAndStack {
    pos: StackPosition,
    max_main: u32,
    ratio: f32,
    ratio_step: f32,
}

impl MainAndStack {
    pub fn side(max_main: u32, ratio: f32, ratio_step: f32) -> Self {
        Self {
            pos: StackPosition::Side,
            max_main,
            ratio,
            ratio_step,
        }
    }

    pub fn bottom(max_main: u32, ratio: f32, ratio_step: f32) -> Self {
        Self {
            pos: StackPosition::Bottom,
            max_main,
            ratio,
            ratio_step,
        }
    }

    fn layout_side(&self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        let n = s.len() as u32;

        if n <= self.max_main || self.max_main == 0 {
            // In both cases we have all windows in a single stack (all main or all secondary)
            r.as_rows(n).iter().zip(s).map(|(r, c)| (*c, *r)).collect()
        } else {
            // We have two stacks so split the secreen in two and then build a stack for each
            let split = ((r.w as f32) * self.ratio) as u32;
            let (main, stack) = r.split_at_width(split).unwrap();

            main.as_rows(self.max_main)
                .into_iter()
                .chain(stack.as_rows(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        }
    }

    fn layout_bottom(&self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        let n = s.len() as u32;

        if n <= self.max_main || self.max_main == 0 {
            r.as_columns(n)
                .iter()
                .zip(s)
                .map(|(r, c)| (*c, *r))
                .collect()
        } else {
            let split = ((r.h as f32) * self.ratio) as u32;
            let (main, stack) = r.split_at_height(split).unwrap();

            main.as_columns(self.max_main)
                .into_iter()
                .chain(stack.as_columns(n.saturating_sub(self.max_main)))
                .zip(s)
                .map(|(r, c)| (*c, r))
                .collect()
        }
    }
}

impl Default for MainAndStack {
    fn default() -> Self {
        Self {
            pos: StackPosition::Side,
            max_main: 1,
            ratio: 0.6,
            ratio_step: 0.1,
        }
    }
}

impl Layout for MainAndStack {
    fn name(&self) -> String {
        match self.pos {
            StackPosition::Side => "SideStack".to_owned(),
            StackPosition::Bottom => "BottomStack".to_owned(),
        }
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        let positions = match self.pos {
            StackPosition::Side => self.layout_side(s, r),
            StackPosition::Bottom => self.layout_bottom(s, r),
        };

        (self, positions)
    }

    fn layout_empty(self: Box<Self>, _r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        (self, vec![])
    }

    fn handle_message(&mut self, m: &Message) {
        if let Some(&ExpandMain) = m.downcast_ref() {
            self.ratio += self.ratio_step;
            if self.ratio > 1.0 {
                self.ratio = 1.0;
            }
        } else if let Some(&ShrinkMain) = m.downcast_ref() {
            self.ratio -= self.ratio_step;
            if self.ratio < 0.0 {
                self.ratio = 0.0;
            }
        } else if let Some(&IncMain(n)) = m.downcast_ref() {
            if n < 0 {
                self.max_main = self.max_main.saturating_sub((-n) as u32);
            } else {
                self.max_main += n as u32;
            }
        } else if let Some(&Rotate) = m.downcast_ref() {
            self.pos = match self.pos {
                StackPosition::Side => StackPosition::Bottom,
                StackPosition::Bottom => StackPosition::Side,
            };
        }
    }
}

/// Wrap an existing layout and reflect its window positions horizontally.
pub struct ReflectHorizontal(Box<dyn Layout>);

impl ReflectHorizontal {
    pub fn new<L>(layout: L) -> Self
    where
        L: Layout + 'static,
    {
        Self(Box::new(layout))
    }

    fn run_reflected<F>(r: Rect, f: F) -> (Box<dyn Layout>, Vec<(Xid, Rect)>)
    where
        F: FnOnce(Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>),
    {
        let mid = r.y + r.h / 2;

        let reflect = |r: Rect| {
            let Rect { x, y, w, h } = r;
            let x = 2 * mid - x;

            Rect { x, y, w, h }
        };

        let (l, rs) = (f)(r);

        (
            Box::new(Self(l)),
            rs.into_iter().map(|(id, r)| (id, reflect(r))).collect(),
        )
    }
}

impl Layout for ReflectHorizontal {
    fn name(&self) -> String {
        format!("ReflectHorizontal<{}>", self.0.name())
    }

    fn layout_workspace(
        self: Box<Self>,
        w: &Workspace<Xid>,
        r: Rect,
    ) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        Self::run_reflected(r, |r| self.0.layout_workspace(w, r))
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        Self::run_reflected(r, |r| self.0.layout(s, r))
    }

    fn layout_empty(self: Box<Self>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        Self::run_reflected(r, |r| self.0.layout_empty(r))
    }

    fn handle_message(&mut self, m: &Message) {
        self.0.handle_message(m)
    }
}

/// Wrap an existing layout and reflect its window positions vertically.
pub struct ReflectVertical(Box<dyn Layout>);

impl ReflectVertical {
    pub fn new<L>(layout: L) -> Self
    where
        L: Layout + 'static,
    {
        Self(Box::new(layout))
    }

    fn run_reflected<F>(r: Rect, f: F) -> (Box<dyn Layout>, Vec<(Xid, Rect)>)
    where
        F: FnOnce(Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>),
    {
        let mid = r.x + r.w / 2;

        let reflect = |r: Rect| {
            let Rect { x, y, w, h } = r;
            let y = 2 * mid - y;

            Rect { x, y, w, h }
        };

        let (l, rs) = (f)(r);

        (
            Box::new(Self(l)),
            rs.into_iter().map(|(id, r)| (id, reflect(r))).collect(),
        )
    }
}

impl Layout for ReflectVertical {
    fn name(&self) -> String {
        format!("ReflectVertical<{}>", self.0.name())
    }

    fn layout_workspace(
        self: Box<Self>,
        w: &Workspace<Xid>,
        r: Rect,
    ) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        Self::run_reflected(r, |r| self.0.layout_workspace(w, r))
    }

    fn layout(self: Box<Self>, s: &Stack<Xid>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        Self::run_reflected(r, |r| self.0.layout(s, r))
    }

    fn layout_empty(self: Box<Self>, r: Rect) -> (Box<dyn Layout>, Vec<(Xid, Rect)>) {
        Self::run_reflected(r, |r| self.0.layout_empty(r))
    }

    fn handle_message(&mut self, m: &Message) {
        self.0.handle_message(m)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        messages::{common::IncMain, AsMessage},
        *,
    };

    #[test]
    fn message_handling() {
        let mut l = MainAndStack::side(1, 0.6, 0.1);

        l.handle_message(&IncMain(2).as_message());

        assert_eq!(l.max_main, 3);
    }
}
