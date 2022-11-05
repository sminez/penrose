//! Layouts for positioning client windows on the screen within a given workspace.
use crate::{
    pure::{geometry::Rect, Stack},
    stack, Xid,
};
use std::{fmt, mem::swap};

mod messages;
mod transformers;

#[doc(inline)]
pub use messages::{IntoMessage, Message};
#[doc(inline)]
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

    /// Generate screen positions for clients on a given [crate::pure::Workspace].
    ///
    /// If you do not need to know the details of the workspace being laid out, you should can use the
    /// default implementation of this methods which calls [Layout::layout] if there are any clients
    /// present and [Layout::layout_empty] if not.
    ///
    /// # Positioning clients
    /// For each client that should be shown on the screen a pair of its [Xid] and a [Rect] should be
    /// provided, indicating the screen position the client should be placed at. To hide a client that
    /// was present in the [crate::pure::Workspace] simply do not provide a position for it. (You may also provide
    /// positions for clients that were not present in the input if you have the [Xid] available.)
    ///
    /// The order in which the ([Xid], [Rect]) pairs are returned determines the stacking order on the
    /// screen. It does not have to match the stack order of the clients within the [crate::pure::Workspace].
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

impl fmt::Display for Box<dyn Layout> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Layout({})", self.name())
    }
}

/// A stack of [Layout] options for use on a particular [crate::pure::Workspace].
///
/// The [Stack] itself acts as a [Layout], deferring all operations to the
/// currently focused Layout.
pub type LayoutStack = Stack<Box<dyn Layout>>;

impl Default for LayoutStack {
    fn default() -> Self {
        stack!(Box::new(crate::builtin::layout::MainAndStack::default()))
    }
}

impl LayoutStack {
    /// Run the currently focused [Layout] and return the positions it generates.
    ///
    /// If the layout being run wants to be replaced with a new layout, swap it
    /// out for the new one in its current position in the [Stack].
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
