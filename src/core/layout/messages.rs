use std::any::Any;

/// A dynamically typed message to be sent to a [crate::core::layout::Layout] for processing
pub struct Message(Box<dyn Any>);

impl Message {
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

/// Marker trait for a type that can be sent as a [crate::core::layout::Message]
pub trait IntoMessage: Any {
    fn into_message(self) -> Message
    where
        Self: Sized,
    {
        Message(Box::new(self))
    }
}
