/// This is where event parsing is handled and conversion of things like ICCCM and EWMH
/// messages to penrose actions is done.
use crate::core::{
    bindings::{KeyCode, MouseEvent},
    client::Client,
    data_types::{Point, Region, WinId},
    xconnection::{Atom, XEvent},
};

use std::{collections::HashMap, str::FromStr};

pub struct WmState<'a> {
    pub(super) client_map: &'a HashMap<WinId, Client>,
    pub(super) focused_client: Option<WinId>,
    pub(super) full_screen_atom: usize,
}

/// Actions that will be carried out by the [WindowManager][1] in response to individual each
/// [XEvent] received from the provided [XConn][2].
///
/// Note that each action is processed independently.
///
/// [1]: crate::core::manager::WindowManager
/// [2]: crate::core::xconnection::XConn
#[derive(Debug, Clone)]
pub enum EventAction {
    /// An X window gained focus
    ClientFocusLost(WinId),
    /// An X window lost focus
    ClientFocusGained(WinId),
    /// An X window had its WM_NAME or _NET_WM_NAME property changed
    ClientNameChanged(WinId, bool),
    /// Move the given client to the workspace at the given index
    ClientToWorkspace(WinId, usize),
    /// An X window was destroyed
    DestroyClient(WinId),
    /// Screens should be redetected
    DetectScreens,
    /// A new X window needs to be mapped
    MapWindow(WinId),
    /// A window is requesting to be moved or resized
    MoveWindow(WinId, Region),
    /// A grabbed keybinding was triggered
    RunKeyBinding(KeyCode),
    /// A grabbed mouse state was triggered
    RunMouseBinding(MouseEvent),
    /// The active client should be set to this id
    SetActiveClient(WinId),
    /// The active workspace should be set to this index
    SetActiveWorkspace(usize),
    /// The active screen should be set based on point location
    SetScreenFromPoint(Option<Point>),
    /// An X window should be set fullscreen
    ToggleClientFullScreen(WinId, bool),
    /// An unknown property was changed on an X window
    UnknownPropertyChange(WinId, String, bool),
}

pub fn process_next_event(event: XEvent, state: WmState<'_>) -> Vec<EventAction> {
    match event {
        // Direct 1-n mappings of XEvents -> EventActions
        XEvent::Destroy { id } => vec![EventAction::DestroyClient(id)],
        XEvent::KeyPress(code) => vec![EventAction::RunKeyBinding(code)],
        XEvent::Leave { id, rpt, .. } => vec![
            EventAction::ClientFocusLost(id),
            EventAction::SetScreenFromPoint(Some(rpt)),
        ],
        XEvent::MouseEvent(evt) => vec![EventAction::RunMouseBinding(evt)],
        XEvent::RandrNotify => vec![EventAction::DetectScreens],
        XEvent::ScreenChange => vec![EventAction::SetScreenFromPoint(None)],

        // Require processing based on current WindowManager state
        XEvent::ClientMessage { id, dtype, data } => {
            process_client_message(state, id, &dtype, &data)
        }
        XEvent::ConfigureNotify { id, r, is_root } => process_configure_notify(id, r, is_root),
        XEvent::Enter { id, rpt, .. } => process_enter_notify(state, id, rpt),
        XEvent::MapRequest { id, ignore } => process_map_request(state, id, ignore),
        XEvent::PropertyNotify { id, atom, is_root } => process_property_notify(id, atom, is_root),
    }
}

fn process_client_message(
    state: WmState<'_>,
    id: WinId,
    dtype: &str,
    data: &[usize],
) -> Vec<EventAction> {
    debug!(
        "GOT CLIENT MESSAGE: id={} atom={} data={:?}",
        id, dtype, data
    );

    match Atom::from_str(&dtype) {
        Ok(Atom::NetActiveWindow) => vec![EventAction::SetActiveClient(id)],
        Ok(Atom::NetCurrentDesktop) => vec![EventAction::SetActiveWorkspace(data[0])],
        Ok(Atom::NetWmDesktop) => vec![EventAction::ClientToWorkspace(id, data[0])],
        Ok(Atom::NetWmState) if data[1..3].contains(&state.full_screen_atom) => {
            // _NET_WM_STATE_ADD == 1, _NET_WM_STATE_TOGGLE == 2
            let should_fullscreen = [1, 2].contains(&data[0]);
            vec![EventAction::ToggleClientFullScreen(id, should_fullscreen)]
        }

        _ => vec![],
    }
}

fn process_configure_notify(id: WinId, r: Region, is_root: bool) -> Vec<EventAction> {
    if is_root {
        vec![EventAction::DetectScreens]
    } else {
        vec![EventAction::MoveWindow(id, r)]
    }
}

fn process_enter_notify(state: WmState<'_>, id: WinId, rpt: Point) -> Vec<EventAction> {
    let mut actions = vec![
        EventAction::ClientFocusGained(id),
        EventAction::SetScreenFromPoint(Some(rpt)),
    ];

    if let Some(current) = state.focused_client {
        if current != id {
            actions.insert(0, EventAction::ClientFocusLost(current));
        }
    }

    actions
}

fn process_map_request(state: WmState<'_>, id: WinId, ignore: bool) -> Vec<EventAction> {
    if ignore || state.client_map.contains_key(&id) {
        vec![]
    } else {
        vec![EventAction::MapWindow(id)]
    }
}

fn process_property_notify(id: WinId, atom: String, is_root: bool) -> Vec<EventAction> {
    match Atom::from_str(&atom) {
        Ok(a) if a == Atom::WmName || a == Atom::NetWmName => {
            vec![EventAction::ClientNameChanged(id, is_root)]
        }
        _ => vec![EventAction::UnknownPropertyChange(id, atom, is_root)],
    }
}
