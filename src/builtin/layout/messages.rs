//! Built-in layout messages.
//!
//! It is not a hard requirement for [Layout] implementations to handle each
//! of the messages provided by this module but wherever possible you should
//! attempt to do so if the semantics of the message make sense for the
//! layout you are writing.

macro_rules! msg {
    ($m:ident) => {
        impl $crate::core::layout::messages::IntoMessage for $m {}
    };
}

/// Alter the number of clients contained in the main area of the [Layout].
pub struct IncMain(pub i8);
msg!(IncMain);

/// Expand the size of the main area of the [Layout].
pub struct ExpandMain;
msg!(ExpandMain);

/// Shrink the size of the main area of the [Layout]
pub struct ShrinkMain;
msg!(ShrinkMain);

/// Rotate the [Layout] to a new orientation
pub struct Rotate;
msg!(Rotate);

/// Mirror the [Layout] over either the horizontal or vertical axis.
pub struct Mirror;
msg!(Mirror);

/// Unwrap a [LayoutTransformer] to return the underlying [Layout].
///
/// Handling of this message is provided automatically by the [LayoutTransformer]
/// trait.
pub struct UnwrapTransformer;
msg!(UnwrapTransformer);

/// A [Message] sent when a [Layout] is no longer visible (e.g. Layout changed on a visible
/// [Workspace] or the workspace itself becoming hidden).
pub struct Hide;
msg!(Hide);

/// A [Message] sent when Penrose is shutting down or restarting.
pub struct ShutDown;
msg!(ShutDown);
