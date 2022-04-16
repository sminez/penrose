//! Common data types used in penrose crates
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod bindings;
pub mod geometry;
pub mod helpers;

/// An X resource ID
pub type Xid = u32;

/// A relative position along the horizontal and vertical axes
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum RelativePosition {
    /// Left of the current position
    Left,
    /// Right of the current position
    Right,
    /// Above the current position
    Above,
    /// Below the current position
    Below,
}

/// Increment / decrement a value
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Change {
    /// increase the value
    More,
    /// decrease the value, possibly clamping
    Less,
}

/// X window border kind
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Border {
    /// window is urgent
    Urgent,
    /// window currently has focus
    Focused,
    /// window does not have focus
    Unfocused,
}
