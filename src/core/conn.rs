//! A platform agnostic backing connection
use crate::{
    core::{Config, State},
    pure::geometry::{Point, Rect},
    x::XEvent,
    Color, Result,
};
use std::hash::Hash;

// TODO: does this want / need to also rework XEvent into something generic?

/// A platform agnostic backing connection
pub trait Conn {
    /// The ID type used to track clients
    type Id: Clone + PartialEq + Eq + Hash;

    // XXX: Lifted directly from XConn

    /// Block and wait for the next event so it can be processed.
    fn next_event(&self) -> Result<XEvent>;
    /// Flush any pending events to the underlying back end.
    fn flush(&self);

    /// The dimensions of each currently available screen.
    fn screen_details(&self) -> Result<Vec<Rect>>;
    /// The current (x, y) coordinate of the mouse cursor.
    fn cursor_position(&self) -> Result<Point>;
    /// Reposition the mouse cursor to the given (x, y) coordinates within the specified window.
    fn warp_pointer(&self, id: Self::Id, x: i16, y: i16) -> Result<()>;

    // TODO: do the other warp pointer methods need to be here so they can be overwritten?

    /// Look up the current dimensions and position of a given client window.
    fn client_geometry(&self, id: Self::Id) -> Result<Rect>;
    /// Ask the X server for the IDs of all currently known client windows
    fn existing_clients(&self) -> Result<Vec<Self::Id>>;

    /// Kill the given client window, closing it.
    fn kill(&self, id: Self::Id) -> Result<()>;
    /// Set input focus to be held by the given client window.
    fn focus(&self, id: Self::Id) -> Result<()>;

    // XXX: Lifted from XConnExt

    /// Update the geometry of a given client based on the given [Rect].
    fn position_client(&self, id: Self::Id, r: Rect) -> Result<()>;
    /// Display a client on the screen at its current position.
    fn show_client(&self, id: Self::Id) -> Result<()>;
    /// Hide a client by unmapping it and setting its WmState to Iconic
    fn hide_client(&self, id: Self::Id) -> Result<()>;

    /// Request the title of a given client window.
    fn client_title(&self, id: Self::Id) -> Result<String>;
    /// Request a window's PID.
    fn client_pid(&self, id: Self::Id) -> Option<u32>;
    /// Check whether or not the given client should be assigned floating status or not.
    fn client_should_float(&self, id: Self::Id, floating_classes: &[String]) -> bool;
    /// Check whether a particular client should be managed as part of our internal state
    fn client_should_be_managed(&self, id: Self::Id) -> bool;

    /// Update the border color of the given client window.
    fn set_client_border_color<C>(&self, id: Self::Id, color: impl Into<Color>) -> Result<()>;

    // FIXME: sort out bounds on Config
    // /// Set the initial window properties for a newly managed window.
    // fn set_initial_properties(&self, id: Self::Id, config: &Config<Self>) -> Result<()>;

    /// Restack the given windows, each one above the last.
    fn restack<'a, I>(&self, ids: I) -> Result<()>
    where
        Self::Id: 'a,
        I: Iterator<Item = &'a Self::Id>;

    // XXX: Actual operations that need to be carried out

    // FIXME: sort out bounds on State
    // /// Handle external requests to focus the specified client
    // fn handle_focus_in(&self, id: Self::Id, state: &mut State<Self>) -> Result<()>;
    // /// Request a client windows's current workspace
    // fn manage_existing_clients(&self, id: Self::Id, state: &mut State<Self>) -> Result<()>;
}
