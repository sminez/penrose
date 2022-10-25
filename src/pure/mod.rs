//! Side effect free management of internal window manager state
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Layout {}
