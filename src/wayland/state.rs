use crate::wayland::handlers::compositor::ClientState;
use smithay::{
    desktop::{
        find_popup_root_surface, get_popup_toplevel_coords, PopupKind, PopupManager, Space, Window,
    },
    input::{Seat, SeatState},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
        wayland_server::{Display, DisplayHandle},
    },
    wayland::{
        compositor::CompositorState,
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::{PopupSurface, XdgShellState},
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};
use std::{ffi::OsString, sync::Arc, time::Instant};

pub const SEAT_NAME: &str = "penrose";

#[derive(Debug)]
pub struct CalloopData {
    pub(crate) state: WaylandState,
    pub(crate) display_handle: DisplayHandle,
}

#[derive(Debug)]
pub(crate) struct SmithayState {
    pub(crate) compositor: CompositorState,
    pub(crate) xdg_shell: XdgShellState,
    pub(crate) shm: ShmState,
    pub(crate) output_manager: OutputManagerState,
    pub(crate) seat: SeatState<WaylandState>,
    pub(crate) data_device: DataDeviceState,
    pub(crate) popups: PopupManager,
}

impl SmithayState {
    fn new(dh: &DisplayHandle) -> Self {
        Self {
            compositor: CompositorState::new::<WaylandState>(dh),
            xdg_shell: XdgShellState::new::<WaylandState>(dh),
            shm: ShmState::new::<WaylandState>(dh, vec![]),
            output_manager: OutputManagerState::new_with_xdg_output::<WaylandState>(dh),
            seat: SeatState::new(),
            data_device: DataDeviceState::new::<WaylandState>(dh),
            popups: PopupManager::default(),
        }
    }
}

#[derive(Debug)]
pub struct WaylandState {
    pub(crate) start_time: Instant,
    pub(crate) socket_name: OsString,
    pub(crate) display_handle: DisplayHandle,
    pub(crate) seat: Seat<Self>,
    pub(crate) space: Space<Window>,
    pub(crate) loop_signal: LoopSignal,
    pub(crate) smithay_state: SmithayState,
}

impl WaylandState {
    pub fn new(event_loop: &mut EventLoop<'_, CalloopData>, disp: Display<Self>) -> Self {
        let start_time = Instant::now();
        let dh = disp.handle();
        let mut smithay_state = SmithayState::new(&dh);

        // A "seat" is collection of input devices (keyboard, mouse, touchpad etc)
        // It typically has a pointer and maintains both keyboard focus and pointer focus
        let mut seat: Seat<Self> = smithay_state.seat.new_wl_seat(&dh, SEAT_NAME);

        // Notify clients that we have a keyboard available. (Assumes a keyboard is always present)
        // TODO: track keyboard attach/detatch
        seat.add_keyboard(Default::default(), 200, 25).unwrap();

        // Notify clients that we have a pointer available. (Assumes a mouse is always present)
        // TODO: track pointer attach/detatch
        seat.add_pointer();

        // A Space represents a 2D place which windows and outputs can be mapped onto.
        // - Windows get a position and stacking order through mapping
        // - Outputs become views on the Space and can be rendered using Space::render_output
        let space = Space::default();
        let socket_name = Self::init_socket(disp, event_loop);
        let loop_signal = event_loop.get_signal();

        Self {
            start_time,
            socket_name,
            display_handle: dh,
            seat,
            space,
            loop_signal,
            smithay_state,
        }
    }

    fn init_socket(disp: Display<Self>, event_loop: &mut EventLoop<'_, CalloopData>) -> OsString {
        let soc = ListeningSocketSource::new_auto().unwrap();
        let socket_name = soc.socket_name().to_os_string();

        event_loop
            .handle()
            .insert_source(soc, move |stream, _, state| {
                state
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("unable to init wayland event source");

        event_loop
            .handle()
            .insert_source(
                Generic::new(disp, Interest::READ, Mode::Level),
                move |_, disp, state| {
                    // SAFETY: we don't drop the display
                    unsafe {
                        disp.get_mut().dispatch_clients(&mut state.state).unwrap();
                    }

                    Ok(PostAction::Continue)
                },
            )
            .expect("failed to add display");

        socket_name
    }

    pub(crate) fn unconstrain_popup(&self, popup: &PopupSurface) {
        let root = match find_popup_root_surface(&PopupKind::Xdg(popup.clone())) {
            Ok(root) => root,
            _ => return,
        };

        let elem = self
            .space
            .elements()
            .find(|w| w.toplevel().wl_surface() == &root);

        let window = match elem {
            Some(window) => window,
            None => return,
        };

        let output = self.space.outputs().next().unwrap();
        let output_geo = self.space.output_geometry(output).unwrap();
        let window_geo = self.space.element_geometry(window).unwrap();

        // The target geometry for the positioner should be relative to its parent's geometry
        let mut target = output_geo;
        target.loc -= get_popup_toplevel_coords(&PopupKind::Xdg(popup.clone()));
        target.loc -= window_geo.loc;

        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }
}
