//! # Penrose: a library for building your very own tiling window manager
pub mod state;

pub use state::{Position, Screen, Stack, State, Workspace};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Only {n_ws} workspaces were provided but at least {n_screens} are required")]
    InsufficientWorkspaces { n_ws: usize, n_screens: usize },

    #[error("there are no screens available")]
    NoScreens,
}

pub type Result<T> = std::result::Result<T, Error>;
