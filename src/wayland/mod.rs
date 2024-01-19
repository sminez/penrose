//! An experimental wayland backend for Penrose using Smithay

mod backend;
mod grabs;
mod handlers;
mod input;
mod state;

pub use state::WaylandState;
