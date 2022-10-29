//! Side effect free management of internal window manager state
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod diff;
pub mod geometry;
pub mod screen;
pub mod stack;
pub mod stack_set;
pub mod workspace;

pub use screen::Screen;
pub use stack::{Position, Stack};
pub use stack_set::StackSet;
pub use workspace::Workspace;

pub(crate) use diff::Diff;

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
