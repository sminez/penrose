//! Hook implementations and helpers for adding to your Penrose window manager
pub mod ewmh;
pub mod startup;
pub mod named_scratchpads;

pub use ewmh::add_ewmh_hooks;
pub use startup::SpawnOnStartup;
