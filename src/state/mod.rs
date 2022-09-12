//! Sie effect free management of internal window manager state
pub mod screen;
pub mod stack;
pub mod state;
pub mod workspace;

pub use screen::Screen;
pub use stack::{Position, Stack};
pub use state::State;
pub use workspace::Workspace;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Layout {}
