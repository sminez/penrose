//! A mock implementation of XConn that is easier to implement for
//! use in tests.
//! This module and its contents are only available when testing.
use crate::{
    Result, WinId,
    core::bindings::{KeySym, MouseState},
    pure::geometry::{Point, Rect},
    x::{
        ClientAttr, ClientConfig, XConn,
        event::{ClientMessage, XEvent},
        property::{Prop, WindowAttributes, WmState},
    },
};

/// All methods on this trait that return a Result unimplemented by
/// default unless an implementation is provided.
/// The `mock_root` method always returns id 0 and `mock_flush` by default is a no-op.
///
/// Any implementation of `MockXConn` will automatically implement `XConn` by forwarding on
/// calls to `$method` to `mock_$method`.
#[allow(unused_variables, missing_docs)]
pub trait MockXConn: Send {
    fn mock_root(&mut self) -> WinId {
        WinId(0)
    }

    fn mock_unordered_screens(&mut self) -> Result<Vec<Rect>> {
        unimplemented!("mock_unordered_screens")
    }

    fn mock_cursor_position(&mut self) -> Result<Point> {
        unimplemented!("mock_cursor_position")
    }

    fn mock_grab(&mut self, keys: &[KeySym], mouse_states: &[MouseState]) -> Result<()> {
        unimplemented!("mock_grab")
    }

    fn mock_capture_next_key(&mut self, _continuations: &[KeySym]) -> Result<()> {
        Ok(())
    }

    fn mock_cancel_capture_next_key(&mut self) -> Result<()> {
        Ok(())
    }

    fn mock_next_event(&mut self) -> Result<XEvent> {
        unimplemented!("mock_next_event")
    }

    fn mock_flush(&mut self) {}

    fn mock_intern_atom(&mut self, atom: &str) -> Result<WinId> {
        unimplemented!("mock_intern_atom")
    }

    fn mock_atom_name(&mut self, xid: WinId) -> Result<String> {
        unimplemented!("mock_atom_name")
    }

    fn mock_client_geometry(&mut self, client: WinId) -> Result<Rect> {
        unimplemented!("mock_client_geometry")
    }

    fn mock_existing_clients(&mut self) -> Result<Vec<WinId>> {
        unimplemented!("mock_existing_clients")
    }

    fn mock_map(&mut self, client: WinId) -> Result<()> {
        unimplemented!("mock_map")
    }

    fn mock_unmap(&mut self, client: WinId) -> Result<()> {
        unimplemented!("mock_unmap")
    }

    fn mock_kill(&mut self, client: WinId) -> Result<()> {
        unimplemented!("mock_kill")
    }

    fn mock_focus(&mut self, client: WinId) -> Result<()> {
        unimplemented!("mock_focus")
    }

    fn mock_get_prop(&mut self, client: WinId, prop_name: &str) -> Result<Option<Prop>> {
        unimplemented!("mock_get_prop")
    }

    fn mock_list_props(&mut self, client: WinId) -> Result<Vec<String>> {
        unimplemented!("mock_list_props")
    }

    fn mock_get_wm_state(&mut self, client: WinId) -> Result<Option<WmState>> {
        unimplemented!("mock_get_wm_state")
    }

    fn mock_get_window_attributes(&mut self, client: WinId) -> Result<WindowAttributes> {
        unimplemented!("mock_get_window_attributes")
    }

    fn mock_set_wm_state(&mut self, client: WinId, wm_state: WmState) -> Result<()> {
        unimplemented!("mock_set_wm_state")
    }

    fn mock_set_prop(&mut self, client: WinId, name: &str, val: Prop) -> Result<()> {
        unimplemented!("mock_set_prop")
    }

    fn mock_delete_prop(&mut self, client: WinId, prop_name: &str) -> Result<()> {
        unimplemented!("mock_delete_prop")
    }

    fn mock_set_client_attributes(&mut self, client: WinId, attrs: &[ClientAttr]) -> Result<()> {
        unimplemented!("mock_set_client_attributes")
    }

    fn mock_set_client_config(&mut self, client: WinId, data: &[ClientConfig]) -> Result<()> {
        unimplemented!("mock_set_client_config")
    }

    fn mock_send_client_message(&mut self, msg: ClientMessage) -> Result<()> {
        unimplemented!("mock_send_client_message")
    }

    fn mock_warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()> {
        unimplemented!("mock_warp_pointer")
    }
}

impl<T> XConn for T
where
    T: MockXConn,
{
    fn root(&mut self) -> WinId {
        self.mock_root()
    }

    fn unordered_screens(&mut self) -> Result<Vec<Rect>> {
        self.mock_unordered_screens()
    }

    fn cursor_position(&mut self) -> Result<Point> {
        self.mock_cursor_position()
    }

    fn grab(&mut self, keys: &[KeySym], mouse_states: &[MouseState]) -> Result<()> {
        self.mock_grab(keys, mouse_states)
    }

    fn capture_next_key(&mut self, continuations: &[KeySym]) -> Result<()> {
        self.mock_capture_next_key(continuations)
    }

    fn cancel_capture_next_key(&mut self) -> Result<()> {
        self.mock_cancel_capture_next_key()
    }

    fn next_event(&mut self) -> Result<XEvent> {
        self.mock_next_event()
    }

    fn flush(&mut self) {
        self.mock_flush()
    }

    fn intern_atom(&mut self, atom: &str) -> Result<WinId> {
        self.mock_intern_atom(atom)
    }

    fn atom_name(&mut self, xid: WinId) -> Result<String> {
        self.mock_atom_name(xid)
    }

    fn client_geometry(&mut self, client: WinId) -> Result<Rect> {
        self.mock_client_geometry(client)
    }

    fn existing_clients(&mut self) -> Result<Vec<WinId>> {
        self.mock_existing_clients()
    }

    fn map(&mut self, client: WinId) -> Result<()> {
        self.mock_map(client)
    }

    fn unmap(&mut self, client: WinId) -> Result<()> {
        self.mock_unmap(client)
    }

    fn kill(&mut self, client: WinId) -> Result<()> {
        self.mock_kill(client)
    }

    fn focus(&mut self, client: WinId) -> Result<()> {
        self.mock_focus(client)
    }

    fn get_prop(&mut self, client: WinId, prop_name: &str) -> Result<Option<Prop>> {
        self.mock_get_prop(client, prop_name)
    }

    fn list_props(&mut self, client: WinId) -> Result<Vec<String>> {
        self.mock_list_props(client)
    }

    fn get_wm_state(&mut self, client: WinId) -> Result<Option<WmState>> {
        self.mock_get_wm_state(client)
    }

    fn get_window_attributes(&mut self, client: WinId) -> Result<WindowAttributes> {
        self.mock_get_window_attributes(client)
    }

    fn set_wm_state(&mut self, client: WinId, wm_state: WmState) -> Result<()> {
        self.mock_set_wm_state(client, wm_state)
    }

    fn set_prop(&mut self, client: WinId, name: &str, val: Prop) -> Result<()> {
        self.mock_set_prop(client, name, val)
    }

    fn delete_prop(&mut self, client: WinId, prop_name: &str) -> Result<()> {
        self.mock_delete_prop(client, prop_name)
    }

    fn set_client_attributes(&mut self, client: WinId, attrs: &[ClientAttr]) -> Result<()> {
        self.mock_set_client_attributes(client, attrs)
    }

    fn set_client_config(&mut self, client: WinId, data: &[ClientConfig]) -> Result<()> {
        self.mock_set_client_config(client, data)
    }

    fn send_client_message(&mut self, msg: ClientMessage) -> Result<()> {
        self.mock_send_client_message(msg)
    }

    fn warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()> {
        self.mock_warp_pointer(id, x, y)
    }
}

/// A stub XConn implementation that doesn't implement _any_ methods.
///
/// Only usable for passing to test code that requires an XConn due to
/// type signatures but is not making use of it.
#[derive(Debug, Default, Clone, Copy)]
pub struct StubXConn;
impl MockXConn for StubXConn {}
