//! # Penrose: a library for building your very own tiling window manager
mod screen;
mod stack;
mod state;
mod workspace;

pub use screen::{Screen, ScreenDetail};
pub use stack::Stack;
pub use state::State;
pub use workspace::Workspace;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Only {n_ws} workspaces were provided but at least {n_screens} are required")]
    InsufficientWorkspaces { n_ws: usize, n_screens: usize },

    #[error("there are no screens available")]
    NoScreens,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Rect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct Layout {}

#[derive(Debug, PartialEq, Eq)]
pub struct Client {}
