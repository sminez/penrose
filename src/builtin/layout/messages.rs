//! Built-in layout messages.
//!
//! It is not a hard requirement for [crate::core::layout::Layout] implementations to handle each
//! of the messages provided by this module but wherever possible you should
//! attempt to do so if the semantics of the message make sense for the
//! layout you are writing.

macro_rules! msg {
    ($m:ident) => {
        impl $crate::core::layout::IntoMessage for $m {}
    };
}

/// Alter the number of clients contained in the main area of the [crate::core::layout::Layout].
pub struct IncMain(pub i8);
msg!(IncMain);

/// Expand the size of the main area of the [crate::core::layout::Layout].
pub struct ExpandMain;
msg!(ExpandMain);

/// Shrink the size of the main area of the [crate::core::layout::Layout]
pub struct ShrinkMain;
msg!(ShrinkMain);

/// Rotate the [crate::core::layout::Layout] to a new orientation
pub struct Rotate;
msg!(Rotate);

/// Mirror the [crate::core::layout::Layout] over either the horizontal or vertical axis.
pub struct Mirror;
msg!(Mirror);

/// Unwrap a [crate::core::layout::LayoutTransformer] to return the underlying [crate::core::layout::Layout].
///
/// Handling of this message is provided automatically by the [crate::core::layout::LayoutTransformer]
/// trait.
pub struct UnwrapTransformer;
msg!(UnwrapTransformer);

/// A [crate::core::layout::Message] sent when a [crate::core::layout::Layout] is no longer visible (e.g.
/// Layout changed on a visible [crate::pure::Workspace] or the workspace itself becoming hidden).
pub struct Hide;
msg!(Hide);
