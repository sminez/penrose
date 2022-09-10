//! Functionality extensions for penrose
//!
//! Most of these extension work by spawning and / or managing external programs as a sub-process.
pub mod dmenu;
pub mod notify_send;
pub mod scratchpad;

#[doc(inline)]
pub use dmenu::*;

#[doc(inline)]
pub use notify_send::*;

#[doc(inline)]
pub use scratchpad::Scratchpad;
