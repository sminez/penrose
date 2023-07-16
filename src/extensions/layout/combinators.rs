//! Higher order combinators for Layouts that allow for composing their behaviour
use crate::{
    core::layout::{Layout, Message},
    pure::{geometry::Rect, Stack},
    Xid,
};
use std::fmt;

/// Conditionally run one of two layouts based on a predicate function.
///
/// This struct implements [Layout] by selecting between the two provided layouts using
/// a predicate function. By default the left layout will be used, switching to the right
/// when the predicate returns false. Examples of predicate functions that might be useful are:
///   - When the screen size being laid out is smaller than a given threshold
///   - When there are more than a given number of clients that need to be laid out
///   - Based on the absolute position of the screen being laid out.
pub struct Conditional {
    name: String,
    left: Box<dyn Layout>,
    right: Box<dyn Layout>,
    should_use_left: fn(&Stack<Xid>, Rect) -> bool,
    left_is_active: bool,
}

impl fmt::Debug for Conditional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Conditional")
            .field("name", &self.name)
            .field("left", &self.left.name())
            .field("right", &self.right.name())
            .field("left_is_active", &self.left_is_active)
            .finish()
    }
}

impl Conditional {
    /// Construct a new [Conditional] layout, selecting from one of two layouts based on
    /// a predicate function.
    pub fn new<L: Layout + 'static, R: Layout + 'static>(
        name: impl Into<String>,
        left: L,
        right: R,
        should_use_left: fn(&Stack<Xid>, Rect) -> bool,
    ) -> Self {
        Self {
            name: name.into(),
            left: Box::new(left),
            right: Box::new(right),
            should_use_left,
            left_is_active: true,
        }
    }

    /// Create a new [Conditional] layout as with `new` but returned as a trait
    /// object ready to be added to your layout stack in config.
    pub fn boxed<L: Layout + 'static, R: Layout + 'static>(
        name: impl Into<String>,
        left: L,
        right: R,
        should_use_left: fn(&Stack<Xid>, Rect) -> bool,
    ) -> Box<dyn Layout> {
        Box::new(Self::new(name, left, right, should_use_left))
    }
}

impl Layout for Conditional {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(Self {
            name: self.name.clone(),
            left: self.left.boxed_clone(),
            right: self.right.boxed_clone(),
            should_use_left: self.should_use_left,
            left_is_active: self.left_is_active,
        })
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        self.left_is_active = (self.should_use_left)(s, r);
        if self.left_is_active {
            self.left.layout(s, r)
        } else {
            self.right.layout(s, r)
        }
    }

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        if self.left_is_active {
            self.left.handle_message(m)
        } else {
            self.right.handle_message(m)
        }
    }
}
