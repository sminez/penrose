//! Layout for window positioning
use crate::{
    core::Xid,
    geometry::Rect,
    state::{Stack, Workspace},
};

use messages::Message;

// TODO: need versions of these with access to state?
pub trait Layout {
    fn name(&self) -> &'static str;

    // TODO: might want / need this to take and return self rather than a mut ref
    //       so that it is possible for layouts to replace themselves with a new one?
    fn layout_workspace(&mut self, w: &Workspace<Xid>, r: Rect) -> Vec<(Xid, Rect)> {
        match &w.stack {
            Some(s) => self.layout(s, r),
            None => self.layout_empty(r),
        }
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> Vec<(Xid, Rect)>;

    fn layout_empty(&mut self, _r: Rect) -> Vec<(Xid, Rect)> {
        vec![]
    }

    fn handle_message(&mut self, m: Message);
}

pub mod messages {
    use std::any::Any;

    /// A dynamically typed message to be sent to a [Layout] for processing
    pub struct Message(Box<dyn Any>);

    impl Message {
        pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
            self.0.downcast_ref()
        }
    }

    /// Marker trait for a type that can be sent as a [Message]
    pub trait AsMessage: Any {
        fn as_message(self) -> Message
        where
            Self: Sized,
        {
            Message(Box::new(self))
        }
    }

    /// Messages for common [Layout] operations.
    pub mod common {
        use super::AsMessage;

        /// Alter the number of clients contained in the main area of the [Layout]
        pub struct IncMain(pub i8);
        impl AsMessage for IncMain {}

        /// Expand the size of the main area of the [Layout]
        pub struct ExpandMain;
        impl AsMessage for ExpandMain {}

        /// Shrink the size of the main area of the [Layout]
        pub struct ShrinkMain;
        impl AsMessage for ShrinkMain {}
    }

    /// Control messages sent by Penrose itself during window manager operation. All layouts
    /// (particularly those that are maintaing additional state) should consider handling these.
    pub mod control {
        use super::AsMessage;

        /// A [Message] sent when a [Layout] is no longer visible (e.g. Layout changed on a visible
        /// [Workspace] or the workspace itself becoming hidden).
        pub struct Hide;
        impl AsMessage for Hide {}

        /// A [Message] sent when Penrose is shutting down or restarting.
        pub struct ShutDown;
        impl AsMessage for ShutDown {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        messages::{common::IncMain, *},
        *,
    };

    struct TestLayout {
        n_main: u8,
    }

    impl Layout for TestLayout {
        fn name(&self) -> &'static str {
            "TestLayout"
        }

        fn layout(&mut self, _s: &Stack<Xid>, _r: Rect) -> Vec<(Xid, Rect)> {
            vec![]
        }

        fn handle_message(&mut self, m: Message) {
            if let Some(&IncMain(n)) = m.downcast_ref() {
                if n < 0 {
                    self.n_main = std::cmp::max(self.n_main.saturating_sub((-n) as u8), 1);
                } else {
                    self.n_main += n as u8;
                }
            }
        }
    }

    #[test]
    fn message_downcast_works() {
        let mut l = TestLayout { n_main: 2 };
        l.handle_message(IncMain(1).as_message());

        assert_eq!(l.n_main, 3);

        l.handle_message(IncMain(-2).as_message());

        assert_eq!(l.n_main, 1);
    }
}
