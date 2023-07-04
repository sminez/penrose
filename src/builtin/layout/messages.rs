//! Built-in layout messages.
//!
//! It is not a hard requirement for [Layout][0] implementations to handle each
//! of the messages provided by this module but wherever possible you should
//! attempt to do so if the semantics of the message make sense for the
//! layout you are writing.
//!
//!   [0]: crate::core::layout::Layout

/// Mark a type as being usable as a [Message][0] for sending to a [Layout][1]
/// ```
/// use penrose::impl_message;
///
/// struct MyMessageType {
///     important_data: u8
/// }
///
/// impl_message!(MyMessageType);
/// ```
///
///   [0]: crate::core::layout::Message
///   [1]: crate::core::layout::Layout
#[macro_export]
macro_rules! impl_message {
    ($m:ident) => {
        impl $crate::core::layout::IntoMessage for $m {}
    };
}

/// Alter the number of clients contained in the main area of the [Layout][0].
///
///   [0]: crate::core::layout::Layout
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct IncMain(pub i8);
impl_message!(IncMain);

/// Expand the size of the main area of the [Layout][0].
///
///   [0]: crate::core::layout::Layout
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ExpandMain;
impl_message!(ExpandMain);

/// Shrink the size of the main area of the [Layout][0]
///
///   [0]: crate::core::layout::Layout
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ShrinkMain;
impl_message!(ShrinkMain);

/// Rotate the [Layout][0] to a new orientation
///
///   [0]: crate::core::layout::Layout
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Rotate;
impl_message!(Rotate);

/// Mirror the [Layout][0] over either the horizontal or vertical axis.
///
///   [0]: crate::core::layout::Layout
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Mirror;
impl_message!(Mirror);

/// Unwrap a [LayoutTransformer][0] to return the underlying [Layout][1].
///
/// Handling of this message is provided automatically by the [LayoutTransformer][0] trait.
///
///   [0]: crate::core::layout::LayoutTransformer
///   [1]: crate::core::layout::Layout
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UnwrapTransformer;
impl_message!(UnwrapTransformer);

/// A [Message][0] sent when a [Layout][1] is no longer visible (e.g. Layout changed on a visible
/// [Workspace][2] or the workspace itself becoming hidden).
///
///   [0]: crate::core::layout::Message
///   [1]: crate::core::layout::Layout
///   [2]: crate::pure::Workspace
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Hide;
impl_message!(Hide);
