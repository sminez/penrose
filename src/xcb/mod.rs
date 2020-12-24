//! Helpers and utilities for using XCB as a back end for penrose
use crate::{
    bindings::{KeyCode, MouseState},
    data_types::{Point, Region, WinId},
    screen::Screen,
    xconnection::{Atom, XEvent},
    Result,
};

pub mod api;
pub mod xconn;

/// A client propert value that can be set.
///
/// Variants correspond to the X property types being set.
pub enum PropVal<'a> {
    /// A slice of interned [`crate::xconnection::Atom`] values
    Atom(&'a [u32]),
    /// A slice of cardinal u32s
    Cardinal(&'a [u32]),
    /// A string valued property
    Str(&'a str),
    /// One or more [`crate::data_types::WinId`] values
    Window(&'a [WinId]),
}

/// A window type to be specified when creating a new window in the X server
pub enum WinType {
    /// A simple hidden stub window for facilitating other API calls
    CheckWin,
    /// A window that receives input only (not queryable)
    InputOnly,
    /// A regular window. The [`crate::xconnection::Atom`] passed should be a
    /// valid _NET_WM_WINDOW_TYPE (this is not enforced)
    InputOutput(Atom),
}

/// Config options for X windows (not all are currently implemented)
pub enum WinConfig {
    /// The border width in pixels
    BorderPx(u32),
    /// Absolute size and position on the screen as a [`core::data_types::Region`]
    Position(Region),
    /// Mark this window as stacking on top of its peers
    StackAbove,
}

impl From<&WinConfig> for Vec<(u16, u32)> {
    fn from(w: &WinConfig) -> Vec<(u16, u32)> {
        match w {
            WinConfig::BorderPx(px) => vec![(xcb::CONFIG_WINDOW_BORDER_WIDTH as u16, *px)],
            WinConfig::Position(region) => {
                let (x, y, w, h) = region.values();
                vec![
                    (xcb::CONFIG_WINDOW_X as u16, x),
                    (xcb::CONFIG_WINDOW_Y as u16, y),
                    (xcb::CONFIG_WINDOW_WIDTH as u16, w),
                    (xcb::CONFIG_WINDOW_HEIGHT as u16, h),
                ]
            }
            WinConfig::StackAbove => {
                vec![(xcb::CONFIG_WINDOW_STACK_MODE as u16, xcb::STACK_MODE_ABOVE)]
            }
        }
    }
}

/// Window attributes for an X11 client window (not all are curently implemented)
pub enum WinAttr {
    /// Border color as an argb hex value
    BorderColor(u32),
    /// Set the pre-defined client event mask
    ClientEventMask,
    /// Set the pre-defined root event mask
    RootEventMask,
}

impl From<&WinAttr> for Vec<(u32, u32)> {
    fn from(w: &WinAttr) -> Vec<(u32, u32)> {
        let client_event_mask = xcb::EVENT_MASK_ENTER_WINDOW
            | xcb::EVENT_MASK_LEAVE_WINDOW
            | xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_STRUCTURE_NOTIFY;

        let root_event_mask = xcb::EVENT_MASK_PROPERTY_CHANGE
            | xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT
            | xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY
            | xcb::EVENT_MASK_BUTTON_MOTION;

        match w {
            WinAttr::BorderColor(c) => vec![(xcb::CW_BORDER_PIXEL, *c)],
            WinAttr::ClientEventMask => vec![(xcb::CW_EVENT_MASK, client_event_mask)],
            WinAttr::RootEventMask => vec![(xcb::CW_EVENT_MASK, root_event_mask)],
        }
    }
}

/**
 * An abstraction layer for talking to the X server using the XCB api.
 *
 * This has been written to be a reasonably close mapping to the underlying
 * C API, but provides several quality of life changes that make consuming
 * the API nicer to work with in Penrose code.
 */
pub trait XcbApi {
    /**
     * Intern an atom by name, returning the corresponding id.
     *
     * Can fail if the atom name is not a known X atom or if there
     * is an issue with communicating with the X server. For known
     * atoms that are included in the [`core::xconnection::Atom`] enum,
     * the [`known_atom`] method should be used instead.
     */
    fn atom(&self, name: &str) -> Result<u32>;

    /**
     * Fetch the id value of a known [`core::xconnection::Atom`] variant.
     *
     * This operation is expected to always succeed as known atoms should
     * either be interned on init of the implementing struct or statically
     * assigned a value in the implementation.
     */
    fn known_atom(&self, atom: Atom) -> u32;

    /// Delete a known property from a window
    fn delete_prop(&self, id: WinId, prop: Atom);
    /// Fetch an [`core::xconnection::Atom`] property for a given window
    fn get_atom_prop(&self, id: WinId, atom: Atom) -> Result<u32>;
    /// Fetch an String property for a given window
    fn get_str_prop(&self, id: WinId, name: &str) -> Result<String>;
    /**
     * Replace a property value on a window.
     *
     * See the documentation for the C level XCB API for the correct property
     * type for each prop.
     */
    fn replace_prop(&self, id: WinId, prop: Atom, val: PropVal);

    /// Create a new client window
    fn create_window(&self, ty: WinType, r: Region, screen: usize, managed: bool) -> Result<WinId>;
    /// Apply a set of config options to a window
    fn configure_window(&self, id: WinId, conf: &[WinConfig]);
    /// The list of currently active clients known to the X server
    fn current_clients(&self) -> Result<Vec<WinId>>;
    /// Destroy the X server state for a given window
    fn destroy_window(&self, id: WinId);
    /// The client that the X server currently considers to be focused
    fn focused_client(&self) -> Result<WinId>;
    /// Send a [`core::xconnection::XEvent::MapRequest`] for the target window
    fn map_window(&self, id: WinId);
    /// Mark the given window as currently having focus in the X server state
    fn mark_focused_window(&self, id: WinId);
    /// Send an event to a client
    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()>;
    /// Set attributes on the target window
    fn set_window_attributes(&self, id: WinId, attrs: &[WinAttr]);
    /// Unmap the target window
    fn unmap_window(&self, id: WinId);
    /// Find the current size and position of the target window
    fn window_geometry(&self, id: WinId) -> Result<Region>;

    /// Query the randr API for current outputs and return the details as penrose
    /// [`core::screen::Screen`] structs.
    fn current_screens(&self) -> Result<Vec<Screen>>;
    /// Query the randr API for current outputs and return the size of each screen
    fn screen_sizes(&self) -> Result<Vec<Region>>;

    /// The current (x, y) position of the cursor relative to the root window
    fn cursor_position(&self) -> Point;
    /// Register intercepts for each given [`core::bindings::KeyCode']
    fn grab_keys(&self, keys: &[&KeyCode]);
    /// Register intercepts for each given [`core::bindings::MouseState']
    fn grab_mouse_buttons(&self, states: &[&MouseState]);
    /// Drop all active intercepts for key combinations
    fn ungrab_keys(&self);
    /// Drop all active intercepts for mouse states
    fn ungrab_mouse_buttons(&self);

    /// Flush pending actions to the X event loop
    fn flush(&self) -> bool;
    /// The current root window ID
    fn root(&self) -> WinId;
    /// Set a pre-defined notify mask for randr events to subscribe to
    fn set_randr_notify_mask(&self) -> Result<()>;
    /**
     * Block until the next event from the X event loop is ready then return it.
     *
     * This method should handle all of the mapping of xcb events to penrose
     * [`core::xconnection::XEvent`] instances, returning None when the event
     * channel from the X server is closed.
     */
    fn wait_for_event(&self) -> Option<XEvent>;
    /// Move the cursor to the given (x, y) position inside the specified window.
    fn warp_cursor(&self, id: WinId, x: usize, y: usize);
}
