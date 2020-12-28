use crate::core::{
    bindings::{KeyCode, MouseEvent},
    client::Client,
    data_types::{Point, WinId},
    xconnection::{Atom, XEvent},
};

use std::{collections::HashMap, str::FromStr};

pub struct WmState<'a> {
    pub(super) client_map: &'a HashMap<WinId, Client>,
    pub(super) focused_client: Option<WinId>,
    pub(super) full_screen_atom: usize,
}

#[derive(Debug, Clone)]
pub enum EventAction {
    ClientFocusLost(WinId),
    ClientFocusGained(WinId),
    ClientNameChanged(WinId),
    DestroyClient(WinId),
    DetectScreens,
    MapWindow(WinId),
    RunKeyBinding(KeyCode),
    RunMouseBinding(MouseEvent),
    SetScreenFromPoint(Option<Point>),
    ToggleClientFullScreen(WinId, bool, bool),
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
        XEvent::ConfigureNotify { is_root, .. } => process_configure_notify(is_root),
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
    let is_full_screen = [data.get(1), data.get(2)].contains(&Some(&state.full_screen_atom));

    match Atom::from_str(&dtype) {
        Ok(Atom::NetWmState) if is_full_screen => {
            if let Some(c) = state.client_map.get(&id) {
                // _NET_WM_STATE_ADD == 1, _NET_WM_STATE_TOGGLE == 2
                let should_fullscreen = [1, 2].contains(&data[0]) && !c.fullscreen;
                vec![EventAction::ToggleClientFullScreen(
                    id,
                    should_fullscreen,
                    c.fullscreen,
                )]
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

fn process_configure_notify(is_root: bool) -> Vec<EventAction> {
    if is_root {
        vec![EventAction::DetectScreens]
    } else {
        vec![]
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
        Ok(a) if !is_root && [Atom::WmName, Atom::NetWmName].contains(&a) => {
            vec![EventAction::ClientNameChanged(id)]
        }
        _ => vec![EventAction::UnknownPropertyChange(id, atom, is_root)],
    }
}
