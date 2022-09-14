//! # Penrose: a library for building your very own tiling window manager
pub mod core;
pub mod geometry;
pub mod state;

pub use crate::core::Xid;
pub use geometry::{Point, Rect};
pub use state::{Position, Screen, Stack, StackSet, Workspace};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Only {n_ws} workspaces were provided but at least {n_screens} are required")]
    InsufficientWorkspaces { n_ws: usize, n_screens: usize },

    #[error("There are no screens available")]
    NoScreens,

    #[error("The given client is not in this State")]
    UnknownClient,
}

pub type Result<T> = std::result::Result<T, Error>;
