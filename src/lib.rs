#[macro_use]
pub mod macros;

pub mod client;
pub mod data_types;
pub mod helpers;
pub mod layout;
pub mod manager;
pub mod screen;
pub mod workspace;
pub mod xconnection;

// top level re-exports
pub use data_types::{ColorScheme, Config};
pub use layout::{Layout, LayoutKind};
pub use manager::WindowManager;
