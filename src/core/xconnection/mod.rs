//! An abstraciton layer for talking to an underlying X server.
//!
//! An implementation of the [XConn] trait is required for running a [WindowManager][1]. The choice
//! of back end (e.g. xlib, xcb...) is an implementation detail that does not surface in the
//! `WindowManager` itself. All low level details of working with the X server should be captured in
//! this trait, though accessing backend specific functionality is possible by writing an impl
//! block for `WindowManager<YourXConn>` if desired.
//!
//! [1]: crate::core::manager::WindowManager
use crate::{
    core::{
        bindings::{KeyBindings, MouseBindings},
        data_types::{Point, Region, WinId},
        screen::Screen,
    },
    draw::Color,
    PenroseError, Result,
};

use std::{cell::Cell, fmt};

pub mod atom;
pub mod event;
pub mod property;

pub use atom::{
    Atom, AtomIter, AUTO_FLOAT_WINDOW_TYPES, EWMH_SUPPORTED_ATOMS, UNMANAGED_WINDOW_TYPES,
};
pub use event::XEvent;
pub use property::{Prop, WmHints, WmNormalHints, WmNormalHintsFlags};

/// A handle on a running X11 connection that we can use for issuing X requests.
///
/// XConn is intended as an abstraction layer to allow for communication with the underlying
/// display system (assumed to be X) using whatever mechanism the implementer wishes. In theory, it
/// should be possible to write an implementation that allows penrose to run on systems not using X
/// as the windowing system but X idioms and high level event types / client interations are
/// assumed.
pub trait XConn {
    /// Hydrate this XConn to restore internal state following serde deserialization
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()>;

    /// Initialise any state required before this connection can be used by the WindowManager.
    ///
    /// This must include checking to see if another window manager is running and return an error
    /// if there is, but other than that there are no other requirements.
    ///
    /// This method is called once during [WindowManager::init][1]
    ///
    /// [1]: crate::core::manager::WindowManager::init
    fn init(&self) -> Result<()>;

    /// Flush pending actions to the X event loop
    fn flush(&self) -> bool;

    /// Wait for the next event from the X server and return it as an [XEvent]
    fn wait_for_event(&self) -> Result<XEvent>;

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

    /// Return the client ID of the [crate::core::client::Client] that currently holds X focus
    fn focused_client(&self) -> WinId;

    /// Mark the given [crate::core::client::Client] as having focus
    fn focus_client(&self, id: WinId);

    /// Change the border color for the given client
    fn set_client_border_color(&self, id: WinId, color: Color);

    /// Notify the X server that we are intercepting the user specified key bindings and prevent
    /// them being passed through to the underlying applications.
    ///
    /// This is what determines which key press events end up being sent through in the main event
    /// loop for the WindowManager.
    fn grab_keys(&self, key_bindings: &KeyBindings<Self>, mouse_bindings: &MouseBindings<Self>)
    where
        Self: Sized;

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

    /// Warp the cursor to be within the specified window. If id == None then behaviour is
    /// definined by the implementor (e.g. warp cursor to active window, warp to center of screen)
    fn warp_cursor(&self, id: Option<WinId>, screen: &Screen);

    /// Run on startup/restart to determine already running windows that we need to track
    fn query_for_active_windows(&self) -> Vec<WinId>;

    /// Query a property for a window by window ID and name.
    ///
    /// Can fail if the property name is invalid or we get a malformed response from xcb.
    fn get_prop(&self, id: WinId, name: &str) -> Result<Prop>;

    /// Return the list of all properties set on the given client window
    ///
    /// Properties should be returned as their string name as would be used to intern the
    /// respective atom.
    fn list_props(&self, id: WinId) -> Result<Vec<String>>;

    /// Intern an X atom by name and return the corresponding ID
    fn intern_atom(&self, atom: &str) -> Result<u32>;

    /// Perform any state cleanup required prior to shutting down the window manager
    fn cleanup(&self);
}

/// A really simple stub implementation of [XConn] to simplify setting up test cases.
///
/// Intended use is to override the mock_* methods that you need for running your test case in order
/// to inject behaviour into a WindowManager instance which is driven by X server state.
/// [StubXConn] will then implement [XConn] and call through to your overwritten methods or the
/// provided default.
///
/// This is being done to avoid providing broken default methods on the real XConn trait that would
/// make writing real impls more error prone if and when new methods are added to the trait.
pub trait StubXConn {
    /// Mocked version of hydrate
    #[cfg(feature = "serde")]
    fn mock_hydrate(&mut self) -> Result<()> {
        Ok(())
    }

    /// Mocked version of init
    fn mock_init(&self) -> Result<()> {
        Ok(())
    }

    /// Mocked version of flush
    fn mock_flush(&self) -> bool {
        true
    }

    /// Mocked version of wait_for_event
    fn mock_wait_for_event(&self) -> Result<XEvent> {
        Err(PenroseError::Raw("mock impl".into()))
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

    /// Mocked version of get_prop
    fn mock_get_prop(&self, _: WinId, prop: &str) -> Result<Prop> {
        if prop == Atom::WmName.as_ref() || prop == Atom::NetWmName.as_ref() {
            Ok(Prop::UTF8String(vec!["mock name".into()]))
        } else {
            Err(PenroseError::Raw("mocked".into()))
        }
    }

    /// Mocked version of list_props
    fn mock_list_props(&self, _: WinId) -> Result<Vec<String>> {
        Ok(vec![])
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
    fn mock_set_client_border_color(&self, _: WinId, _: Color) {}
    /// Mocked version of grab_keys
    fn mock_grab_keys(&self, _: &KeyBindings<Self>, _: &MouseBindings<Self>)
    where
        Self: Sized,
    {
    }
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
    #[cfg(feature = "serde")]
    fn hydrate(&mut self) -> Result<()> {
        self.mock_hydrate()
    }

    fn init(&self) -> Result<()> {
        self.mock_init()
    }

    fn flush(&self) -> bool {
        self.mock_flush()
    }

    fn wait_for_event(&self) -> Result<XEvent> {
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

    fn set_client_border_color(&self, id: WinId, color: Color) {
        self.mock_set_client_border_color(id, color)
    }

    fn grab_keys(&self, key_bindings: &KeyBindings<Self>, mouse_bindings: &MouseBindings<Self>) {
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

    fn get_prop(&self, id: WinId, name: &str) -> Result<Prop> {
        self.mock_get_prop(id, name)
    }

    fn list_props(&self, id: WinId) -> Result<Vec<String>> {
        self.mock_list_props(id)
    }

    fn intern_atom(&self, atom: &str) -> Result<u32> {
        self.mock_intern_atom(atom)
    }

    fn cleanup(&self) {
        self.mock_cleanup()
    }
}

/// A dummy [XConn] implementation for testing
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MockXConn {
    screens: Vec<Screen>,
    #[cfg_attr(feature = "serde", serde(skip))]
    events: Cell<Vec<XEvent>>,
    focused: Cell<WinId>,
    unmanaged_ids: Vec<WinId>,
}

impl fmt::Debug for MockXConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockXConn")
            .field("screens", &self.screens)
            .field("remaining_events", &self.remaining_events())
            .field("focused", &self.focused.get())
            .field("unmanaged_ids", &self.unmanaged_ids)
            .finish()
    }
}

impl MockXConn {
    /// Set up a new [MockXConn] with pre-defined [Screen]s and an event stream to pull from
    pub fn new(screens: Vec<Screen>, events: Vec<XEvent>, unmanaged_ids: Vec<WinId>) -> Self {
        MockXConn {
            screens,
            events: Cell::new(events),
            focused: Cell::new(0),
            unmanaged_ids,
        }
    }
    fn remaining_events(&self) -> Vec<XEvent> {
        let remaining = self.events.replace(vec![]);
        self.events.set(remaining.clone());
        remaining
    }
}

impl StubXConn for MockXConn {
    fn mock_wait_for_event(&self) -> Result<XEvent> {
        let mut remaining = self.events.replace(vec![]);
        if remaining.is_empty() {
            return Err(PenroseError::Raw("mock conn closed".into()));
        }
        let next = remaining.remove(0);
        self.events.set(remaining);
        Ok(next)
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
