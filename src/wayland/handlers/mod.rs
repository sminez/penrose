//! Smithay *Handler trait implementations
use crate::wayland::state::WaylandState;
use smithay::{
    delegate_compositor, delegate_data_device, delegate_output, delegate_seat, delegate_shm,
    delegate_xdg_shell,
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    wayland::selection::{
        data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
            ServerDndGrabHandler,
        },
        SelectionHandler,
    },
};

pub(crate) mod compositor;
pub(crate) mod xdg_shell;

delegate_data_device!(WaylandState);
delegate_compositor!(WaylandState);
delegate_xdg_shell!(WaylandState);
delegate_output!(WaylandState);
delegate_seat!(WaylandState);
delegate_shm!(WaylandState);

impl SeatHandler for WaylandState {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.smithay_state.seat
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&Self::KeyboardFocus>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
}

impl SelectionHandler for WaylandState {
    type SelectionUserData = ();
}

impl ClientDndGrabHandler for WaylandState {}
impl ServerDndGrabHandler for WaylandState {}

impl DataDeviceHandler for WaylandState {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.smithay_state.data_device
    }
}
