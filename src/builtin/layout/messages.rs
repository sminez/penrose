//! Built-in layout messages

macro_rules! msg {
    ($m:ident) => {
        impl $crate::core::layout::messages::IntoMessage for $m {}
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
