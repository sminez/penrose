//! Side effect free management of internal window manager state
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

mod diff;
pub mod geometry;
mod screen;
mod stack;
mod stack_set;
mod workspace;

#[doc(inline)]
pub use screen::Screen;
#[doc(inline)]
pub use stack::{Position, Stack};
#[doc(inline)]
pub use stack_set::StackSet;
#[doc(inline)]
pub use workspace::Workspace;

pub(crate) use diff::{Diff, Snapshot};
pub(crate) use screen::ScreenClients;

#[cfg(test)]
pub(crate) use stack_set::tests::test_xid_stack_set;

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
