use std::any::Any;
use std::fmt;

/// A dynamically typed message to be sent to a [Layout][0] for processing.
///
/// See the [IntoMessage] trait for how to mark a type as being usable as a [Message].
///
///   [0]: crate::core::layout::Layout
pub struct Message(Box<dyn Any>);

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message").finish()
    }
}

impl Message {
    /// Check to see whether this [Message] is a particular type
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

/// Marker trait for a type that can be sent as a [Message].
///
/// The [impl_message][crate::impl_message] macro can be used to easily implement this trait and mark
/// a type as being usable as a layout message:
/// ```
/// use penrose::impl_message;
///
/// struct MyMessage;
/// impl_message!(MyMessage);
/// ```
pub trait IntoMessage: Any {
    /// Wrap this value as a dynamically typed message for sending to a layout
    fn into_message(self) -> Message
    where
        Self: Sized,
    {
        Message(Box::new(self))
    }
}
