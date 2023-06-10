//! A mock implementation of XConn that is easier to implement for
//! use in tests.
//! This module and its contents are only available when testing.
use crate::{
    core::bindings::{KeyCode, MouseState},
    pure::geometry::{Point, Rect},
    x::{
        event::{ClientMessage, XEvent},
        property::{Prop, WindowAttributes, WmState},
        ClientAttr, ClientConfig, XConn,
    },
    Result, Xid,
};

/// All methods on this trait that return a Result unimplemented by
/// default unless an implementation is provided.
/// The `mock_root` method always returns id 0 and `mock_flush` by default is a no-op.
///
/// Any implementation of `MockXConn` will automatically implement `XConn` by forwarding on
/// calls to `$method` to `mock_$method`.
#[allow(unused_variables)]
pub trait MockXConn {
    fn mock_root(&self) -> Xid {
        Xid(0)
    }

    fn mock_screen_details(&self) -> Result<Vec<Rect>> {
        unimplemented!("mock_screen_details")
    }

    fn mock_cursor_position(&self) -> Result<Point> {
        unimplemented!("mock_cursor_position")
    }

    fn mock_grab(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()> {
        unimplemented!("mock_grab")
    }

    fn mock_next_event(&self) -> Result<XEvent> {
        unimplemented!("mock_next_event")
    }

    fn mock_flush(&self) {}

    fn mock_intern_atom(&self, atom: &str) -> Result<Xid> {
        unimplemented!("mock_intern_atom")
    }

    fn mock_atom_name(&self, xid: Xid) -> Result<String> {
        unimplemented!("mock_atom_name")
    }

    fn mock_client_geometry(&self, client: Xid) -> Result<Rect> {
        unimplemented!("mock_client_geometry")
    }

    fn mock_existing_clients(&self) -> Result<Vec<Xid>> {
        unimplemented!("mock_existing_clients")
    }

    fn mock_map(&self, client: Xid) -> Result<()> {
        unimplemented!("mock_map")
    }

    fn mock_unmap(&self, client: Xid) -> Result<()> {
        unimplemented!("mock_unmap")
    }

    fn mock_kill(&self, client: Xid) -> Result<()> {
        unimplemented!("mock_kill")
    }

    fn mock_focus(&self, client: Xid) -> Result<()> {
        unimplemented!("mock_focus")
    }

    fn mock_get_prop(&self, client: Xid, prop_name: &str) -> Result<Option<Prop>> {
        unimplemented!("mock_get_prop")
    }

    fn mock_list_props(&self, client: Xid) -> Result<Vec<String>> {
        unimplemented!("mock_list_props")
    }

    fn mock_get_wm_state(&self, client: Xid) -> Result<Option<WmState>> {
        unimplemented!("mock_get_wm_state")
    }

    fn mock_get_window_attributes(&self, client: Xid) -> Result<WindowAttributes> {
        unimplemented!("mock_get_window_attributes")
    }

    fn mock_set_wm_state(&self, client: Xid, wm_state: WmState) -> Result<()> {
        unimplemented!("mock_set_wm_state")
    }

    fn mock_set_prop(&self, client: Xid, name: &str, val: Prop) -> Result<()> {
        unimplemented!("mock_set_prop")
    }

    fn mock_delete_prop(&self, client: Xid, prop_name: &str) -> Result<()> {
        unimplemented!("mock_delete_prop")
    }

    fn mock_set_client_attributes(&self, client: Xid, attrs: &[ClientAttr]) -> Result<()> {
        unimplemented!("mock_set_client_attributes")
    }

    fn mock_set_client_config(&self, client: Xid, data: &[ClientConfig]) -> Result<()> {
        unimplemented!("mock_set_client_config")
    }

    fn mock_send_client_message(&self, msg: ClientMessage) -> Result<()> {
        unimplemented!("mock_send_client_message")
    }

    fn mock_warp_pointer(&self, id: Xid, x: i16, y: i16) -> Result<()> {
        unimplemented!("mock_warp_pointer")
    }
}

impl<T> XConn for T
where
    T: MockXConn,
{
    fn root(&self) -> Xid {
        self.mock_root()
    }

    fn screen_details(&self) -> Result<Vec<Rect>> {
        self.mock_screen_details()
    }

    fn cursor_position(&self) -> Result<Point> {
        self.mock_cursor_position()
    }

    fn grab(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()> {
        self.mock_grab(key_codes, mouse_states)
    }

    fn next_event(&self) -> Result<XEvent> {
        self.mock_next_event()
    }

    fn flush(&self) {
        self.mock_flush()
    }

    fn intern_atom(&self, atom: &str) -> Result<Xid> {
        self.mock_intern_atom(atom)
    }

    fn atom_name(&self, xid: Xid) -> Result<String> {
        self.mock_atom_name(xid)
    }

    fn client_geometry(&self, client: Xid) -> Result<Rect> {
        self.mock_client_geometry(client)
    }

    fn existing_clients(&self) -> Result<Vec<Xid>> {
        self.mock_existing_clients()
    }

    fn map(&self, client: Xid) -> Result<()> {
        self.mock_map(client)
    }

    fn unmap(&self, client: Xid) -> Result<()> {
        self.mock_unmap(client)
    }

    fn kill(&self, client: Xid) -> Result<()> {
        self.mock_kill(client)
    }

    fn focus(&self, client: Xid) -> Result<()> {
        self.mock_focus(client)
    }

    fn get_prop(&self, client: Xid, prop_name: &str) -> Result<Option<Prop>> {
        self.mock_get_prop(client, prop_name)
    }

    fn list_props(&self, client: Xid) -> Result<Vec<String>> {
        self.mock_list_props(client)
    }

    fn get_wm_state(&self, client: Xid) -> Result<Option<WmState>> {
        self.mock_get_wm_state(client)
    }

    fn get_window_attributes(&self, client: Xid) -> Result<WindowAttributes> {
        self.mock_get_window_attributes(client)
    }

    fn set_wm_state(&self, client: Xid, wm_state: WmState) -> Result<()> {
        self.mock_set_wm_state(client, wm_state)
    }

    fn set_prop(&self, client: Xid, name: &str, val: Prop) -> Result<()> {
        self.mock_set_prop(client, name, val)
    }

    fn delete_prop(&self, client: Xid, prop_name: &str) -> Result<()> {
        self.mock_delete_prop(client, prop_name)
    }

    fn set_client_attributes(&self, client: Xid, attrs: &[ClientAttr]) -> Result<()> {
        self.mock_set_client_attributes(client, attrs)
    }

    fn set_client_config(&self, client: Xid, data: &[ClientConfig]) -> Result<()> {
        self.mock_set_client_config(client, data)
    }

    fn send_client_message(&self, msg: ClientMessage) -> Result<()> {
        self.mock_send_client_message(msg)
    }

    fn warp_pointer(&self, id: Xid, x: i16, y: i16) -> Result<()> {
        self.mock_warp_pointer(id, x, y)
    }
}
