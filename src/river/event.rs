//! The events [RiverConn][super::RiverConn] delivers to penrose.
//!
//! This is deliberately much smaller than river's protocol: the sequence events, window
//! dimensions and object bookkeeping are all consumed inside the conn and never reach the
//! window manager. What is left is the set of things penrose acts on.
use crate::core::{
    bindings::{KeySym, MouseEvent},
    conn::{ConnEvent, WinId},
};
use std::fmt;

/// An event from the river compositor that penrose acts on.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RiverEvent {
    /// A bound key was pressed.
    KeyPress(KeySym),

    /// A key press was eaten by a capture without matching any binding.
    ///
    /// This is river's `ate_unbound_key`, which carries no keysym: all it means is "a key was
    /// eaten and it was not one of yours", which is the signal to abandon a key sequence.
    UnboundKey,

    /// A bound mouse button was pressed or released.
    MouseEvent(MouseEvent),

    /// A new window was created.
    ///
    /// Delivered at the end of the batch of events it arrived in, so that everything river had
    /// to say about the window (its app id, title, parent) is already known when the window
    /// manager is told about it.
    WindowOpened(WinId),

    /// A window was closed by the compositor.
    WindowClosed(WinId),

    /// A window changed its title.
    Title(WinId),

    /// A window changed its app id.
    AppId(WinId),

    /// A window asked to be made fullscreen, or to leave fullscreen.
    FullscreenRequested(WinId, bool),

    /// An output was added, removed, or changed its position, dimensions or usable area.
    ScreenChange,

    /// The pointer entered a window.
    PointerFocus(WinId),

    /// A window was interacted with: clicked, touched or similar.
    Interaction(WinId),

    /// River has taken window management away from us and we should exit.
    ///
    /// This is either the hot swap in another window manager or the compositor shutting down.
    Finished,
}

impl fmt::Display for RiverEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyPress(k) => write!(f, "KeyPress({}, {})", k.mask, k.keysym),
            Self::UnboundKey => write!(f, "UnboundKey"),
            Self::MouseEvent(e) => write!(f, "MouseEvent({})", e.data.id),
            Self::WindowOpened(id) => write!(f, "WindowOpened({id})"),
            Self::WindowClosed(id) => write!(f, "WindowClosed({id})"),
            Self::Title(id) => write!(f, "Title({id})"),
            Self::AppId(id) => write!(f, "AppId({id})"),
            Self::FullscreenRequested(id, full) => write!(f, "FullscreenRequested({id}, {full})"),
            Self::ScreenChange => write!(f, "ScreenChange"),
            Self::PointerFocus(id) => write!(f, "PointerFocus({id})"),
            Self::Interaction(id) => write!(f, "Interaction({id})"),
            Self::Finished => write!(f, "Finished"),
        }
    }
}

impl ConnEvent for RiverEvent {
    fn requires_pointer_warp(&self) -> bool {
        // Warping to a window the pointer is already in would fight with the user's hand, which
        // is the same reason the X11 backend excludes Enter.
        !matches!(self, Self::PointerFocus(_) | Self::Interaction(_))
    }
}
