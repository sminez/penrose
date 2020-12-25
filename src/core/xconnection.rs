/*!
 * An abstraciton layer for talking to an underlying X server.
 *
 * An implementation of the [`XConn`] trait is required for running a
 * [`crate::core::manager::WindowManager`]. The choice of back end (e.g. xlib, xcb...) is an
 * implementation detail that does not surface in the WindowManager itself. All low level details
 * of working with the X server should be captured in this trait.
 */
use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
        data_types::{Point, Region, WinId},
        screen::Screen,
    },
    Result,
};

use std::cell::Cell;

use strum::*;

/**
 * A Penrose internal representation of X atoms.
 *
 * Atom names are shared between all X11 API libraries so this enum allows us to get a little bit
 * of type safety around their use. Implementors of [`XConn`] should accept any variant of [`Atom`]
 * that they are passed by client code.
 */
#[derive(AsRefStr, EnumString, EnumIter, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Atom {
    #[strum(serialize = "ATOM")]
    /// ATOM
    Atom,
    #[strum(serialize = "ATOM_WINDOW")]
    /// ATOM_WINDOW
    Window,
    #[strum(serialize = "ATOM_CARDINAL")]
    /// ATOM_CARDINAL
    Cardinal,
    #[strum(serialize = "MANAGER")]
    /// MANAGER
    Manager,
    #[strum(serialize = "UTF8_STRING")]
    /// UTF8_STRING
    UTF8String,
    #[strum(serialize = "WM_CLASS")]
    /// WM_CLASS
    WmClass,
    #[strum(serialize = "WM_DELETE_WINDOW")]
    /// WM_DELETE_WINDOW
    WmDeleteWindow,
    #[strum(serialize = "WM_PROTOCOLS")]
    /// WM_PROTOCOLS
    WmProtocols,
    #[strum(serialize = "WM_STATE")]
    /// WM_STATE
    WmState,
    #[strum(serialize = "WM_NAME")]
    /// WM_NAME
    WmName,
    #[strum(serialize = "WM_TAKE_FOCUS")]
    /// WM_TAKE_FOCUS
    WmTakeFocus,
    #[strum(serialize = "_NET_ACTIVE_WINDOW")]
    /// _NET_ACTIVE_WINDOW
    NetActiveWindow,
    #[strum(serialize = "_NET_CLIENT_LIST")]
    /// _NET_CLIENT_LIST
    NetClientList,
    #[strum(serialize = "_NET_CURRENT_DESKTOP")]
    /// _NET_CURRENT_DESKTOP
    NetCurrentDesktop,
    #[strum(serialize = "_NET_DESKTOP_NAMES")]
    /// _NET_DESKTOP_NAMES
    NetDesktopNames,
    #[strum(serialize = "_NET_NUMBER_OF_DESKTOPS")]
    /// _NET_NUMBER_OF_DESKTOPS
    NetNumberOfDesktops,
    #[strum(serialize = "_NET_SUPPORTED")]
    /// _NET_SUPPORTED
    NetSupported,
    #[strum(serialize = "_NET_SUPPORTING_WM_CHECK")]
    /// _NET_SUPPORTING_WM_CHECK
    NetSupportingWmCheck,
    #[strum(serialize = "_NET_SYSTEM_TRAY_OPCODE")]
    /// _NET_SYSTEM_TRAY_OPCODE
    NetSystemTrayOpcode,
    #[strum(serialize = "_NET_SYSTEM_TRAY_ORIENTATION")]
    /// _NET_SYSTEM_TRAY_ORIENTATION
    NetSystemTrayOrientation,
    #[strum(serialize = "_NET_SYSTEM_TRAY_ORIENTATION_HORZ")]
    /// _NET_SYSTEM_TRAY_ORIENTATION_HORZ
    NetSystemTrayOrientationHorz,
    #[strum(serialize = "_NET_SYSTEM_TRAY_S0")]
    /// _NET_SYSTEM_TRAY_S0
    NetSystemTrayS0,
    #[strum(serialize = "_NET_WM_DESKTOP")]
    /// _NET_WM_DESKTOP
    NetWmDesktop,
    #[strum(serialize = "_NET_WM_NAME")]
    /// _NET_WM_NAME
    NetWmName,
    #[strum(serialize = "_NET_WM_STATE")]
    /// _NET_WM_STATE
    NetWmState,
    #[strum(serialize = "_NET_WM_STATE_FULLSCREEN")]
    /// _NET_WM_STATE_FULLSCREEN
    NetWmStateFullscreen,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE")]
    /// _NET_WM_WINDOW_TYPE
    NetWmWindowType,
    #[strum(serialize = "_XEMBED")]
    /// _XEMBED
    XEmbed,
    #[strum(serialize = "_XEMBED_INFO")]
    /// _XEMBED_INFO
    XEmbedInfo,

    // Window Types
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DESKTOP")]
    /// _NET_WM_WINDOW_TYPE_DESKTOP
    NetWindowTypeDesktop,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DOCK")]
    /// _NET_WM_WINDOW_TYPE_DOCK
    NetWindowTypeDock,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_TOOLBAR")]
    /// _NET_WM_WINDOW_TYPE_TOOLBAR
    NetWindowTypeToolbar,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_MENU")]
    /// _NET_WM_WINDOW_TYPE_MENU
    NetWindowTypeMenu,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_UTILITY")]
    /// _NET_WM_WINDOW_TYPE_UTILITY
    NetWindowTypeUtility,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_SPLASH")]
    /// _NET_WM_WINDOW_TYPE_SPLASH
    NetWindowTypeSplash,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DIALOG")]
    /// _NET_WM_WINDOW_TYPE_DIALOG
    NetWindowTypeDialog,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU")]
    /// _NET_WM_WINDOW_TYPE_DROPDOWN_MENU
    NetWindowTypeDropdownMenu,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_POPUP_MENU")]
    /// _NET_WM_WINDOW_TYPE_POPUP_MENU
    NetWindowTypePopupMenu,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_NOTIFICATION")]
    /// _NET_WM_WINDOW_TYPE_NOTIFICATION
    NetWindowTypeNotification,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_COMBO")]
    /// _NET_WM_WINDOW_TYPE_COMBO
    NetWindowTypeCombo,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_DND")]
    /// _NET_WM_WINDOW_TYPE_DND
    NetWindowTypeDnd,
    #[strum(serialize = "_NET_WM_WINDOW_TYPE_NORMAL")]
    /// _NET_WM_WINDOW_TYPE_NORMAL
    NetWindowTypeNormal,
}

// Clients with one of these window types will be auto floated
pub(crate) const AUTO_FLOAT_WINDOW_TYPES: &[Atom] = &[
    Atom::NetWindowTypeDesktop,
    Atom::NetWindowTypeDialog,
    Atom::NetWindowTypeDock,
    Atom::NetWindowTypeDropdownMenu,
    Atom::NetWindowTypeMenu,
    Atom::NetWindowTypeNotification,
    Atom::NetWindowTypePopupMenu,
    Atom::NetWindowTypeSplash,
    Atom::NetWindowTypeToolbar,
    Atom::NetWindowTypeUtility,
];

pub(crate) const UNMANAGED_WINDOW_TYPES: &[Atom] =
    &[Atom::NetWindowTypeDock, Atom::NetWindowTypeToolbar];

pub(crate) const EWMH_SUPPORTED_ATOMS: &[Atom] = &[
    Atom::NetActiveWindow,
    Atom::NetClientList,
    Atom::NetCurrentDesktop,
    Atom::NetDesktopNames,
    Atom::NetNumberOfDesktops,
    Atom::NetSupported,
    Atom::NetSupportingWmCheck,
    // Atom::NetSystemTrayS0,
    // Atom::NetSystemTrayOpcode,
    // Atom::NetSystemTrayOrientationHorz,
    Atom::NetWmName,
    Atom::NetWmState,
    Atom::NetWmStateFullscreen,
    Atom::NetWmWindowType,
];

/**
 * Wrapper around the low level XCB event types that require casting to work with.
 * Not all event fields are extracted so check the XCB documentation and update
 * accordingly if you need access to something that isn't currently passed through
 * to the WindowManager event loop.
 *
 * <https://tronche.com/gui/x/xlib/events/types.html>
 * <https://github.com/rtbo/rust-xcb/xml/xproto.xml>
 */
#[derive(Debug, Clone)]
pub enum XEvent {
    /// xcb docs: <https://www.mankier.com/3/xcb_button_press_event_t>
    /// xcb docs: <https://www.mankier.com/3/xcb_motion_notify_event_t>
    MouseEvent(MouseEvent),

    /// xcb docs: <https://www.mankier.com/3/xcb_input_device_key_press_event_t>
    KeyPress(KeyCode),

    /// xcb docs: <https://www.mankier.com/3/xcb_map_request_event_t>
    MapRequest {
        /// The ID of the window that wants to be mapped
        id: WinId,
        /// Whether or not the WindowManager should handle this window.
        ignore: bool,
    },

    /// xcb docs: <https://www.mankier.com/3/xcb_enter_notify_event_t>
    Enter {
        /// The ID of the window that was entered
        id: WinId,
        /// Absolute coordinate of the event
        rpt: Point,
        /// Coordinate of the event relative to top-left of the window itself
        wpt: Point,
    },

    /// xcb docs: <https://www.mankier.com/3/xcb_enter_notify_event_t>
    Leave {
        /// The ID of the window that was left
        id: WinId,
        /// Absolute coordinate of the event
        rpt: Point,
        /// Coordinate of the event relative to top-left of the window itself
        wpt: Point,
    },

    /// xcb docs: <https://www.mankier.com/3/xcb_destroy_notify_event_t>
    Destroy {
        /// The ID of the window being destroyed
        id: WinId,
    },

    /// xcb docs: <https://www.mankier.com/3/xcb_randr_screen_change_notify_event_t>
    ScreenChange,

    /// xcb docs: <https://www.mankier.com/3/xcb_randr_notify_event_t>
    RandrNotify,

    /// xcb docs: <https://www.mankier.com/3/xcb_configure_notify_event_t>
    ConfigureNotify {
        /// The ID of the window that had a property changed
        id: WinId,
        /// The new window size
        r: Region,
        /// Is this window the root window?
        is_root: bool,
    },

    /// xcb docs: <https://www.mankier.com/3/xcb_property_notify_event_t>
    PropertyNotify {
        /// The ID of the window that had a property changed
        id: WinId,
        /// The property that changed
        atom: String,
        /// Is this window the root window?
        is_root: bool,
    },

    /// <https://www.mankier.com/3/xcb_client_message_event_t>
    ClientMessage {
        /// The ID of the window that sent the message
        id: WinId,
        /// The data type being set
        dtype: String,
        /// The data itself
        data: Vec<usize>,
    },
}

/**
 * A handle on a running X11 connection that we can use for issuing X requests.
 *
 * XConn is intended as an abstraction layer to allow for communication with the underlying display
 * system (assumed to be X) using whatever mechanism the implementer wishes. In theory, it should
 * be possible to write an implementation that allows penrose to run on systems not using X as the
 * windowing system but X idioms and high level event types / client interations are assumed.
 **/
pub trait XConn {
    /// Flush pending actions to the X event loop
    fn flush(&self) -> bool;

    /// Wait for the next event from the X server and return it as an [`XEvent`]
    fn wait_for_event(&self) -> Option<XEvent>;

    /// Determine the currently connected CRTCs and return their details
    fn current_outputs(&self) -> Vec<Screen>;

    /// Determine the current (x,y) position of the cursor relative to the root window.
    fn cursor_position(&self) -> Point;

    /// Reposition the window identified by 'id' to the specifed region
    fn position_window(&self, id: WinId, r: Region, border: u32, stack_above: bool);

    /// Raise the window to the top of the stack so it renders above peers
    fn raise_window(&self, id: WinId);

    /// Mark the given window as newly created
    fn mark_new_window(&self, id: WinId);

    /// Map a window to the display. Called each time a map_notify event is received
    fn map_window(&self, id: WinId);

    /// Unmap a window from the display. Called each time an unmap_notify event is received
    fn unmap_window(&self, id: WinId);

    /// Send an X event to the target window
    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()>;

    /// Return the client ID of the [`crate::core::client::Client`] that currently holds X focus
    fn focused_client(&self) -> WinId;

    /// Mark the given [`crate::core::client::Client`] as having focus
    fn focus_client(&self, id: WinId);

    /// Change the border color for the given client
    fn set_client_border_color(&self, id: WinId, color: u32);

    /**
     * Notify the X server that we are intercepting the user specified key bindings
     * and prevent them being passed through to the underlying applications. This
     * is what determines which key press events end up being sent through in the
     * main event loop for the WindowManager.
     */
    fn grab_keys(&self, key_bindings: &KeyBindings, mouse_bindings: &MouseBindings);

    /// Set required EWMH properties to ensure compatability with external programs
    fn set_wm_properties(&self, workspaces: &[&str]);

    /// Update the root window properties with the current desktop details
    fn update_desktops(&self, workspaces: &[&str]);

    /// Update the root window properties with the current client details
    fn update_known_clients(&self, clients: &[WinId]);

    /// Update which desktop is currently focused
    fn set_current_workspace(&self, wix: usize);

    /// Set the WM_NAME prop of the root window
    fn set_root_window_name(&self, name: &str);

    /// Update which desktop a client is currently on
    fn set_client_workspace(&self, id: WinId, wix: usize);

    /// Toggle the fullscreen state of the given client ID with the X server
    fn toggle_client_fullscreen(&self, id: WinId, client_is_fullscreen: bool);

    /// Determine whether the target window should be tiled or allowed to float
    fn window_should_float(&self, id: WinId, floating_classes: &[&str]) -> bool;

    /// Check to see if this window is one that we should be handling or not
    fn is_managed_window(&self, id: WinId) -> bool;

    /// Return the current (x, y, w, h) dimensions of the requested window
    fn window_geometry(&self, id: WinId) -> Result<Region>;

    /**
     * Warp the cursor to be within the specified window. If id == None then behaviour is
     * definined by the implementor (e.g. warp cursor to active window, warp to center of screen)
     */
    fn warp_cursor(&self, id: Option<WinId>, screen: &Screen);

    /// Run on startup/restart to determine already running windows that we need to track
    fn query_for_active_windows(&self) -> Vec<WinId>;

    /**
     * Query a string property for a window by window ID and poperty name.
     * Can fail if the property name is invalid or we get a malformed response from xcb.
     */
    fn str_prop(&self, id: u32, name: &str) -> Result<String>;

    /**
     * Fetch an atom prop by name for a particular window ID
     * Can fail if the property name is invalid or we get a malformed response from xcb.
     */
    fn atom_prop(&self, id: u32, name: &str) -> Result<u32>;

    /// Intern an X atom by name and return the corresponding ID
    fn intern_atom(&self, atom: &str) -> Result<u32>;

    /// Perform any state cleanup required prior to shutting down the window manager
    fn cleanup(&self);
}

/**
 * A really simple stub implementation of [`XConn`] to simplify setting up test cases.
 *
 * Intended use is to override the mock_* methods that you need for running your test case in order
 * to inject behaviour into a WindowManager instance which is driven by X server state.
 * [`StubXConn`] will then implement [`XConn`] and call through to your overwritten methods or the
 * provided default.
 *
 * This is being done to avoid providing broken default methods on the real XConn trait that would
 * make writing real impls more error prone if and when new methods are added to the trait.
 */
pub trait StubXConn {
    /// Mocked version of flush
    fn mock_flush(&self) -> bool {
        true
    }

    /// Mocked version of wait_for_event
    fn mock_wait_for_event(&self) -> Option<XEvent> {
        None
    }

    /// Mocked version of current_outputs
    fn mock_current_outputs(&self) -> Vec<Screen> {
        vec![]
    }

    /// Mocked version of cursor_position
    fn mock_cursor_position(&self) -> Point {
        Point::new(0, 0)
    }

    /// Mocked version of send_client_event
    fn mock_send_client_event(&self, _: WinId, _: &str) -> Result<()> {
        Ok(())
    }

    /// Mocked version of focused_client
    fn mock_focused_client(&self) -> WinId {
        0
    }

    /// Mocked version of window_should_float
    fn mock_window_should_float(&self, _: WinId, _: &[&str]) -> bool {
        false
    }

    /// Mocked version of is_managed_window
    fn mock_is_managed_window(&self, _: WinId) -> bool {
        true
    }

    /// Mocked version of window_geometry
    fn mock_window_geometry(&self, _: WinId) -> Result<Region> {
        Ok(Region::new(0, 0, 0, 0))
    }

    /// Mocked version of query_for_active_windows
    fn mock_query_for_active_windows(&self) -> Vec<WinId> {
        Vec::new()
    }

    /// Mocked version of str_prop
    fn mock_str_prop(&self, _: u32, name: &str) -> Result<String> {
        Ok(String::from(name))
    }

    /// Mocked version of atom_prop
    fn mock_atom_prop(&self, id: u32, _: &str) -> Result<u32> {
        Ok(id)
    }

    /// Mocked version of intern_atom
    fn mock_intern_atom(&self, _: &str) -> Result<u32> {
        Ok(0)
    }

    /// Mocked version of warp_cursor
    fn mock_warp_cursor(&self, _: Option<WinId>, _: &Screen) {}
    /// Mocked version of focus_client
    fn mock_focus_client(&self, _: WinId) {}
    /// Mocked version of position_window
    fn mock_position_window(&self, _: WinId, _: Region, _: u32, _: bool) {}
    /// Mocked version of raise_window
    fn mock_raise_window(&self, _: WinId) {}
    /// Mocked version of mark_new_window
    fn mock_mark_new_window(&self, _: WinId) {}
    /// Mocked version of map_window
    fn mock_map_window(&self, _: WinId) {}
    /// Mocked version of unmap_window
    fn mock_unmap_window(&self, _: WinId) {}
    /// Mocked version of set_client_border_color
    fn mock_set_client_border_color(&self, _: WinId, _: u32) {}
    /// Mocked version of grab_keys
    fn mock_grab_keys(&self, _: &KeyBindings, _: &MouseBindings) {}
    /// Mocked version of set_wm_properties
    fn mock_set_wm_properties(&self, _: &[&str]) {}
    /// Mocked version of update_desktops
    fn mock_update_desktops(&self, _: &[&str]) {}
    /// Mocked version of update_known_clients
    fn mock_update_known_clients(&self, _: &[WinId]) {}
    /// Mocked version of set_current_workspace
    fn mock_set_current_workspace(&self, _: usize) {}
    /// Mocked version of set_root_window_name
    fn mock_set_root_window_name(&self, _: &str) {}
    /// Mocked version of set_client_workspace
    fn mock_set_client_workspace(&self, _: WinId, _: usize) {}
    /// Mocked version of toggle_client_fullscreen
    fn mock_toggle_client_fullscreen(&self, _: WinId, _: bool) {}
    /// Mocked version of cleanup
    fn mock_cleanup(&self) {}
}

impl<T> XConn for T
where
    T: StubXConn,
{
    fn flush(&self) -> bool {
        self.mock_flush()
    }

    fn wait_for_event(&self) -> Option<XEvent> {
        self.mock_wait_for_event()
    }

    fn current_outputs(&self) -> Vec<Screen> {
        self.mock_current_outputs()
    }

    fn cursor_position(&self) -> Point {
        self.mock_cursor_position()
    }

    fn position_window(&self, id: WinId, r: Region, border: u32, stack_above: bool) {
        self.mock_position_window(id, r, border, stack_above)
    }

    fn raise_window(&self, id: WinId) {
        self.mock_raise_window(id)
    }

    fn mark_new_window(&self, id: WinId) {
        self.mock_mark_new_window(id)
    }

    fn map_window(&self, id: WinId) {
        self.mock_map_window(id)
    }

    fn unmap_window(&self, id: WinId) {
        self.mock_unmap_window(id)
    }

    fn send_client_event(&self, id: WinId, atom_name: &str) -> Result<()> {
        self.mock_send_client_event(id, atom_name)
    }

    fn focused_client(&self) -> WinId {
        self.mock_focused_client()
    }

    fn focus_client(&self, id: WinId) {
        self.mock_focus_client(id)
    }

    fn set_client_border_color(&self, id: WinId, color: u32) {
        self.mock_set_client_border_color(id, color)
    }

    fn grab_keys(&self, key_bindings: &KeyBindings, mouse_bindings: &MouseBindings) {
        self.mock_grab_keys(key_bindings, mouse_bindings)
    }

    fn set_wm_properties(&self, workspaces: &[&str]) {
        self.mock_set_wm_properties(workspaces)
    }

    fn update_desktops(&self, workspaces: &[&str]) {
        self.mock_update_desktops(workspaces)
    }

    fn update_known_clients(&self, clients: &[WinId]) {
        self.mock_update_known_clients(clients)
    }

    fn set_current_workspace(&self, wix: usize) {
        self.mock_set_current_workspace(wix)
    }

    fn set_root_window_name(&self, name: &str) {
        self.mock_set_root_window_name(name)
    }

    fn set_client_workspace(&self, id: WinId, wix: usize) {
        self.mock_set_client_workspace(id, wix)
    }

    fn toggle_client_fullscreen(&self, id: WinId, client_is_fullscreen: bool) {
        self.mock_toggle_client_fullscreen(id, client_is_fullscreen)
    }

    fn window_should_float(&self, id: WinId, floating_classes: &[&str]) -> bool {
        self.mock_window_should_float(id, floating_classes)
    }

    fn is_managed_window(&self, id: WinId) -> bool {
        self.mock_is_managed_window(id)
    }

    fn window_geometry(&self, id: WinId) -> Result<Region> {
        self.mock_window_geometry(id)
    }

    fn warp_cursor(&self, id: Option<WinId>, screen: &Screen) {
        self.mock_warp_cursor(id, screen)
    }

    fn query_for_active_windows(&self) -> Vec<WinId> {
        self.mock_query_for_active_windows()
    }

    fn str_prop(&self, id: u32, name: &str) -> Result<String> {
        self.mock_str_prop(id, name)
    }

    fn atom_prop(&self, id: u32, name: &str) -> Result<u32> {
        self.mock_atom_prop(id, name)
    }

    fn intern_atom(&self, atom: &str) -> Result<u32> {
        self.mock_intern_atom(atom)
    }

    fn cleanup(&self) {
        self.mock_cleanup()
    }
}

/// A dummy [`XConn`] implementation for testing
pub struct MockXConn {
    screens: Vec<Screen>,
    events: Cell<Vec<XEvent>>,
    focused: Cell<WinId>,
    unmanaged_ids: Vec<WinId>,
}

impl MockXConn {
    /// Set up a new [`MockXConn`] with pre-defined [`Screen`]s and an event stream to pull from
    pub fn new(screens: Vec<Screen>, events: Vec<XEvent>, unmanaged_ids: Vec<WinId>) -> Self {
        MockXConn {
            screens,
            events: Cell::new(events),
            focused: Cell::new(0),
            unmanaged_ids,
        }
    }
}

impl StubXConn for MockXConn {
    fn mock_wait_for_event(&self) -> Option<XEvent> {
        let mut remaining = self.events.replace(vec![]);
        if remaining.is_empty() {
            return None;
        }
        let next = remaining.remove(0);
        self.events.set(remaining);
        Some(next)
    }

    fn mock_current_outputs(&self) -> Vec<Screen> {
        self.screens.clone()
    }

    fn mock_focused_client(&self) -> WinId {
        self.focused.get()
    }

    fn mock_focus_client(&self, id: WinId) {
        self.focused.replace(id);
    }

    fn mock_is_managed_window(&self, id: WinId) -> bool {
        !self.unmanaged_ids.contains(&id)
    }
}
