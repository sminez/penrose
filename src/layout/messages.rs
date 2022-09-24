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

macro_rules! msg {
    ($m:ident) => {
        impl $crate::layout::messages::AsMessage for $m {}
    };
}

/// Messages for common [Layout] operations.
pub mod common {
    /// Alter the number of clients contained in the main area of the [Layout]
    pub struct IncMain(pub i8);
    msg!(IncMain);

    /// Expand the size of the main area of the [Layout]
    pub struct ExpandMain;
    msg!(ExpandMain);

    /// Shrink the size of the main area of the [Layout]
    pub struct ShrinkMain;
    msg!(ShrinkMain);

    /// Rotate the [Layout] to a new orientation
    pub struct Rotate;
    msg!(Rotate);

    /// Unwrap a [LayoutTransformer] to return the underlying [Layout]
    pub struct UnwrapTransformer;
    msg!(UnwrapTransformer);
}

/// Control messages sent by Penrose itself during window manager operation. All layouts
/// (particularly those that are maintaing additional state) should consider handling these.
pub mod control {
    /// A [Message] sent when a [Layout] is no longer visible (e.g. Layout changed on a visible
    /// [Workspace] or the workspace itself becoming hidden).
    pub struct Hide;
    msg!(Hide);

    /// A [Message] sent when Penrose is shutting down or restarting.
    pub struct ShutDown;
    msg!(ShutDown);
}
