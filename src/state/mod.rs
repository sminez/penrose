//! Sie effect free management of internal window manager state
pub mod screen;
pub mod stack;
pub mod stack_set;
pub mod workspace;

pub use screen::Screen;
pub use stack::{Position, Stack};
pub use stack_set::StackSet;
pub use workspace::Workspace;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Layout {}
