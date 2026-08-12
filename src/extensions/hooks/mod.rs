//! Hook implementations and helpers for adding to your Penrose window manager
pub mod default_workspaces;
pub mod manage;
pub mod named_scratchpads;

// EWMH properties, atoms and X11 events have no counterpart on other backends:
// see river-design.md §1 and §9.
#[cfg(feature = "x11rb")]
pub mod ewmh;
#[cfg(feature = "x11rb")]
pub mod startup;
#[cfg(feature = "x11rb")]
pub mod window_swallowing;

pub use named_scratchpads::{NamedScratchPad, ToggleNamedScratchPad, add_named_scratchpads};

#[cfg(feature = "x11rb")]
pub use ewmh::add_ewmh_hooks;
#[cfg(feature = "x11rb")]
pub use startup::SpawnOnStartup;
#[cfg(feature = "x11rb")]
pub use window_swallowing::WindowSwallowing;
