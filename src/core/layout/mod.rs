//! Layouts for positioning client windows on the screen within a given workspace.
use crate::{
    core::layout::messages::common::{ExpandMain, IncMain, Rotate, ShrinkMain},
    pure::{geometry::Rect, Stack},
    stack, Xid,
};
use std::{fmt, mem::swap};

pub mod messages;
pub mod transformers;

pub use messages::{IntoMessage, Message};
pub use transformers::LayoutTransformer;

// TODO: Do I also need versions of the layout methods that have access to the overall X state as well?
//       That would allow for doing more involved layouts that were aware of things like the specific
//       program running in each client window or the properties on each client.
//       The layouts I'm writing initially are all fairly simple from that perspective so there is no
//       need for it to start, and adding it later is possible using the same default imple deferral
//       approach used in `layout_workspace` if it _is_ needed in future without it needing to be a
//       breaking API change to the trait.

/// A [Layout] is responsible for positioning a [Stack] of clients in a given coordinate space denoting
/// the dimensions of users display.
///
/// Mutating the state of a Layout is possible by sending it a [Message] which can then either modify
/// the existing layout (e.g. increase the number of clients positioned in a "main" area) or replace
/// the existing Layout with a new one. There is no requirement to be able to handle all message types.
pub trait Layout {
    /// A short display name for this Layout, appropriate for rendering in a status bar as an indicator
    /// of which layout is currently being used.
    fn name(&self) -> String;

    /// Provide a clone of this [Layout] wrapped as a trait object. (Trait objects can not require
    /// Clone directly)
    fn boxed_clone(&self) -> Box<dyn Layout>;

    /// Generate screen positions for clients on a given [Workspace].
    ///
    /// If you do not need to know the details of the workspace being laid out, you should can use the
    /// default implementation of this methods which calls [Layout::layout] if there are any clients
    /// present and [Layout::layout_empty] if not.
    ///
    /// # Positioning clients
    /// For each client that should be shown on the screen a pair of its [Xid] and a [Rect] should be
    /// provided, indicating the screen position the client should be placed at. To hide a client that
    /// was present in the [Workspace] simply do not provide a position for it. (You may also provide
    /// positions for clients that were not present in the input if you have the [Xid] available.)
    ///
    /// The order in which the ([Xid], [Rect]) pairs are returned determines the stacking order on the
    /// screen. It does not have to match the stack order of the clients within the [Workspace].
    ///
    /// # Returning a new layout
    /// When a layout is run it may optionally replace itself with a new [Layout]. If `Some(layout)`
    /// is returned from this method, it will be swapped out for the current one after the provided
    /// positions have been applied.
    #[allow(clippy::type_complexity)]
    fn layout_workspace(
        &mut self,
        _tag: &str,
        stack: &Option<Stack<Xid>>,
        r: Rect,
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        match stack {
            Some(s) => self.layout(s, r),
            None => self.layout_empty(r),
        }
    }

    /// Generate screen positions for clients from a given [Stack].
    ///
    /// See [Layout::layout_workspace] for details of how positions should be returned.
    #[allow(clippy::type_complexity)]
    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>);

    /// Generate screen positions for an empty [Stack].
    ///
    /// See [Layout::layout_workspace] for details of how positions should be returned.
    #[allow(clippy::type_complexity)]
    fn layout_empty(&mut self, _r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (None, vec![])
    }

    /// Process a dynamic [Message].
    ///
    /// See the trait level docs for details on what is possible with messages.
    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>>;
}

impl Clone for Box<dyn Layout> {
    fn clone(&self) -> Self {
        self.boxed_clone()
    }
}

impl fmt::Debug for Box<dyn Layout> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layout")
            .field("name", &self.name())
            .finish()
    }
}

/// A stack of [Layout] options for use on a particular [Workspace].
///
/// The [Stack] itself acts as a [Layout], deferring all operations to the
/// currently focused Layout.
pub type LayoutStack = Stack<Box<dyn Layout>>;

impl Default for LayoutStack {
    fn default() -> Self {
        stack!(Box::new(MainAndStack::default()))
    }
}

impl LayoutStack {
    pub fn run_and_replace<F>(&mut self, f: F) -> Vec<(Xid, Rect)>
    where
        F: FnOnce(&mut Box<dyn Layout>) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>),
    {
        let (new_focus, rs) = (f)(&mut self.focus);

        if let Some(mut new) = new_focus {
            self.swap_focus(&mut new);
        }

        rs
    }

    /// Send the given [Message] to the currently active [Layout].
    pub fn handle_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        let m = m.into_message();

        if let Some(mut new) = self.focus.handle_message(&m) {
            swap(&mut self.focus, &mut new);
        }
    }

    /// Send the given [Message] to every [Layout] in this stack rather that just the
    /// currently active one.
    pub fn broadcast_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        let m = m.into_message();

        for l in self.iter_mut() {
            if let Some(mut new) = l.handle_message(&m) {
                swap(l, &mut new);
            }
        }
    }
}

impl Layout for LayoutStack {
    fn name(&self) -> String {
        self.focus.name()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(self.clone())
    }

    fn layout_workspace(
        &mut self,
        tag: &str,
        stack: &Option<Stack<Xid>>,
        r: Rect,
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (
            None,
            self.run_and_replace(|l| l.layout_workspace(tag, stack, r)),
        )
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (None, self.run_and_replace(|l| l.layout(s, r)))
    }

    fn layout_empty(&mut self, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (None, self.run_and_replace(|l| l.layout_empty(r)))
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        let new_focus = self.focus.handle_message(m);

        if let Some(mut new) = new_focus {
            self.swap_focus(&mut new);
        }

        None
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
    pub fn side(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::side_unboxed(max_main, ratio, ratio_step))
    }

    pub fn side_unboxed(max_main: u32, ratio: f32, ratio_step: f32) -> Self {
        Self {
            pos: StackPosition::Side,
            max_main,
            ratio,
            ratio_step,
        }
    }

    pub fn bottom(max_main: u32, ratio: f32, ratio_step: f32) -> Box<dyn Layout> {
        Box::new(Self::bottom_unboxed(max_main, ratio, ratio_step))
    }

    pub fn bottom_unboxed(max_main: u32, ratio: f32, ratio_step: f32) -> Self {
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

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(*self)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        let positions = match self.pos {
            StackPosition::Side => self.layout_side(s, r),
            StackPosition::Bottom => self.layout_bottom(s, r),
        };

        (None, positions)
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
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

        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        messages::{common::IncMain, IntoMessage},
        *,
    };

    #[test]
    fn message_handling() {
        let mut l = MainAndStack::side_unboxed(1, 0.6, 0.1);

        l.handle_message(&IncMain(2).into_message());

        assert_eq!(l.max_main, 3);
    }
}
