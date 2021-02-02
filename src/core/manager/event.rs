/// This is where event parsing is handled and conversion of things like ICCCM and EWMH
/// messages to penrose actions is done.
use crate::core::{
    bindings::{KeyCode, MouseEvent},
    client::Client,
    data_types::{Point, Region},
    xconnection::{Atom, XConn, XEvent, Xid},
};

use std::{collections::HashMap, str::FromStr};

pub(super) struct WmState<'a, X>
where
    X: XConn,
{
    pub(super) conn: &'a X,
    pub(super) client_map: &'a HashMap<Xid, Client>,
    pub(super) focused_client: Option<Xid>,
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
    ClientFocusLost(Xid),
    /// An X window lost focus
    ClientFocusGained(Xid),
    /// An X window had its WM_NAME or _NET_WM_NAME property changed
    ClientNameChanged(Xid, bool),
    /// Move the given client to the workspace at the given index
    ClientToWorkspace(Xid, usize),
    /// An X window was destroyed
    DestroyClient(Xid),
    /// Screens should be redetected
    DetectScreens,
    /// A new X window needs to be mapped
    MapWindow(Xid),
    /// A client is requesting to be moved: honoured if the client is floating
    MoveClientIfFloating(Xid, Region),
    /// A grabbed keybinding was triggered
    RunKeyBinding(KeyCode),
    /// A grabbed mouse state was triggered
    RunMouseBinding(MouseEvent),
    /// The active client should be set to this id
    SetActiveClient(Xid),
    /// The active workspace should be set to this index
    SetActiveWorkspace(usize),
    /// The active screen should be set based on point location
    SetScreenFromPoint(Option<Point>),
    /// An X window should be set fullscreen
    ToggleClientFullScreen(Xid, bool),
    /// An unknown property was changed on an X window
    UnknownPropertyChange(Xid, String, bool),
}

pub(super) fn process_next_event<X>(event: XEvent, state: WmState<'_, X>) -> Vec<EventAction>
where
    X: XConn,
{
    match event {
        // Direct 1-n mappings of XEvents -> EventActions
        XEvent::Destroy { id } => vec![EventAction::DestroyClient(id)],
        XEvent::Expose { .. } => vec![], // TODO: work out if this needs handling in the WindowManager
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
        XEvent::ConfigureRequest { id, r, is_root } => process_configure_request(id, r, is_root),
        XEvent::Enter { id, rpt, .. } => process_enter_notify(state, id, rpt),
        XEvent::MapRequest { id, ignore } => process_map_request(state, id, ignore),
        XEvent::PropertyNotify { id, atom, is_root } => process_property_notify(id, atom, is_root),
    }
}

fn process_client_message<X>(
    state: WmState<'_, X>,
    id: Xid,
    dtype: &str,
    data: &[usize],
) -> Vec<EventAction>
where
    X: XConn,
{
    debug!(
        "GOT CLIENT MESSAGE: id={} atom={} data={:?}",
        id, dtype, data
    );

    let is_fullscreen = |data: &[usize]| {
        data.iter()
            .map(|&a| state.conn.atom_name(a as u32))
            .flatten()
            .any(|s| &s == Atom::NetWmStateFullscreen.as_ref())
    };

    match Atom::from_str(&dtype) {
        Ok(Atom::NetActiveWindow) => vec![EventAction::SetActiveClient(id)],
        Ok(Atom::NetCurrentDesktop) => vec![EventAction::SetActiveWorkspace(data[0])],
        Ok(Atom::NetWmDesktop) => vec![EventAction::ClientToWorkspace(id, data[0])],
        Ok(Atom::NetWmState) if is_fullscreen(&data[1..3]) => {
            // _NET_WM_STATE_ADD == 1, _NET_WM_STATE_TOGGLE == 2
            let should_fullscreen = [1, 2].contains(&data[0]);
            vec![EventAction::ToggleClientFullScreen(id, should_fullscreen)]
        }

        _ => vec![],
    }
}

fn process_configure_notify(_id: Xid, _r: Region, is_root: bool) -> Vec<EventAction> {
    if is_root {
        vec![EventAction::DetectScreens]
    } else {
        vec![]
    }
}

fn process_configure_request(id: Xid, r: Region, is_root: bool) -> Vec<EventAction> {
    if !is_root {
        vec![EventAction::MoveClientIfFloating(id, r)]
    } else {
        vec![]
    }
}

fn process_enter_notify<X>(state: WmState<'_, X>, id: Xid, rpt: Point) -> Vec<EventAction>
where
    X: XConn,
{
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

fn process_map_request<X>(state: WmState<'_, X>, id: Xid, ignore: bool) -> Vec<EventAction>
where
    X: XConn,
{
    if ignore || state.client_map.contains_key(&id) {
        vec![]
    } else {
        vec![EventAction::MapWindow(id)]
    }
}

fn process_property_notify(id: Xid, atom: String, is_root: bool) -> Vec<EventAction> {
    match Atom::from_str(&atom) {
        Ok(a) if a == Atom::WmName || a == Atom::NetWmName => {
            vec![EventAction::ClientNameChanged(id, is_root)]
        }
        _ => vec![EventAction::UnknownPropertyChange(id, atom, is_root)],
    }
}
