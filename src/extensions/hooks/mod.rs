//! Hook implementations and helpers for adding to your Penrose window manager
pub mod default_workspaces;
pub mod ewmh;
pub mod manage;
pub mod named_scratchpads;
pub mod startup;
pub mod window_swallowing;

pub use ewmh::add_ewmh_hooks;
pub use named_scratchpads::{NamedScratchPad, ToggleNamedScratchPad, add_named_scratchpads};
pub use startup::SpawnOnStartup;
pub use window_swallowing::WindowSwallowing;
