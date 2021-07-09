/// This is where event parsing is handled and conversion of things like ICCCM and EWMH
/// messages to penrose actions is done.
use crate::core::{
    bindings::{KeyCode, MouseEvent},
    client::Client,
    data_types::{Point, Region},
    hooks::HookName,
    manager::WindowManager,
    xconnection::{
        Atom, ClientMessage, ConfigureEvent, PointerChange, PropertyEvent, XConn, XEvent, Xid,
    },
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

impl<'a, X> WmState<'a, X>
where
    X: XConn,
{
    pub(super) fn new(manager: &'a WindowManager<X>) -> Self {
        Self {
            conn: &manager.conn,
            client_map: &manager.client_map,
            focused_client: manager.focused_client,
        }
    }
}

/// Actions that will be carried out by the [WindowManager][1] in response to individual each
/// [XEvent] received from the provided [XConn][2].
///
/// Note that each action is processed independently.
///
/// [1]: crate::core::manager::WindowManager
/// [2]: crate::core::xconnection::XConn
#[non_exhaustive]
#[must_use = "Generated event actions must be handled"]
#[derive(Debug)]
pub enum EventAction<'a> {
    /// An X window lost focus
    ClientFocusLost(Xid),
    /// An X window gained focus
    ClientFocusGained(Xid),
    /// An X window had its WM_NAME or _NET_WM_NAME property changed
    ClientNameChanged(Xid, bool),
    /// Move the given client to the workspace at the given index
    ClientToWorkspace(Xid, usize),
    /// An X window was destroyed
    DestroyClient(Xid),
    /// Screens should be redetected
    DetectScreens,
    /// A client should have focus
    FocusIn(Xid),
    /// The workspace on each screen should be layed out again
    LayoutVisible,
    /// A new X window needs to be mapped
    MapWindow(Xid),
    /// A client is requesting to be moved: honoured if the client is floating
    MoveClientIfFloating(Xid, Region),
    /// The named hook should now be run
    RunHook(HookName<'a>),
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
    /// A window is becoming unmapped
    Unmap(Xid),
}

pub(super) fn process_next_event<'a, 'b, X>(
    event: XEvent,
    state: WmState<'a, X>,
) -> Vec<EventAction<'b>>
where
    'b: 'a,
    X: XConn,
{
    match event {
        // Direct 1-n mappings of XEvents -> EventActions
        XEvent::Destroy(id) => vec![EventAction::DestroyClient(id)],
        XEvent::Expose(_) => vec![], // FIXME: work out if this needs handling in the WindowManager
        XEvent::FocusIn(id) => vec![EventAction::FocusIn(id)],
        XEvent::KeyPress(code) => vec![EventAction::RunKeyBinding(code)],
        XEvent::Leave(p) => vec![
            EventAction::ClientFocusLost(p.id),
            EventAction::SetScreenFromPoint(Some(p.abs)),
        ],
        XEvent::MouseEvent(evt) => vec![EventAction::RunMouseBinding(evt)],
        XEvent::RandrNotify => vec![EventAction::DetectScreens],
        XEvent::ScreenChange => vec![EventAction::SetScreenFromPoint(None)],
        XEvent::UnmapNotify(id) => vec![EventAction::Unmap(id)],

        // Require processing based on current WindowManager state
        XEvent::ClientMessage(msg) => process_client_message(state, msg),
        XEvent::ConfigureNotify(evt) => process_configure_notify(evt),
        XEvent::ConfigureRequest(evt) => process_configure_request(evt),
        XEvent::Enter(p) => process_enter_notify(state, p),
        XEvent::MapRequest(id, override_redirect) => {
            process_map_request(state, id, override_redirect)
        }
        XEvent::PropertyNotify(evt) => process_property_notify(evt),
    }
}

fn process_client_message<'a, 'b, X>(
    state: WmState<'a, X>,
    msg: ClientMessage,
) -> Vec<EventAction<'b>>
where
    X: XConn,
{
    let data = msg.data();
    trace!(id = msg.id, dtype = ?msg.dtype, ?data, "got client message");

    let is_fullscreen = |data: &[u32]| {
        data.iter()
            .map(|&a| state.conn.atom_name(a))
            .flatten()
            .any(|s| s == Atom::NetWmStateFullscreen.as_ref())
    };

    match Atom::from_str(&msg.dtype) {
        Ok(Atom::NetActiveWindow) => vec![EventAction::SetActiveClient(msg.id)],
        Ok(Atom::NetCurrentDesktop) => vec![EventAction::SetActiveWorkspace(data.as_usize()[0])],
        Ok(Atom::NetWmDesktop) => vec![EventAction::ClientToWorkspace(msg.id, data.as_usize()[0])],
        Ok(Atom::NetWmState) if is_fullscreen(&data.as_u32()[1..3]) => {
            // _NET_WM_STATE_ADD == 1, _NET_WM_STATE_TOGGLE == 2
            let should_fullscreen = [1, 2].contains(&data.as_usize()[0]);
            vec![EventAction::ToggleClientFullScreen(
                msg.id,
                should_fullscreen,
            )]
        }

        _ => vec![],
    }
}

fn process_configure_notify<'a>(evt: ConfigureEvent) -> Vec<EventAction<'a>> {
    if evt.is_root {
        vec![EventAction::DetectScreens]
    } else {
        vec![]
    }
}

fn process_configure_request<'a>(evt: ConfigureEvent) -> Vec<EventAction<'a>> {
    if !evt.is_root {
        vec![EventAction::MoveClientIfFloating(evt.id, evt.r)]
    } else {
        vec![]
    }
}

fn process_enter_notify<'a, 'b, X>(state: WmState<'a, X>, p: PointerChange) -> Vec<EventAction<'b>>
where
    X: XConn,
{
    let mut actions = vec![
        EventAction::ClientFocusGained(p.id),
        EventAction::SetScreenFromPoint(Some(p.abs)),
    ];

    if let Some(current) = state.focused_client {
        if current != p.id {
            actions.insert(0, EventAction::ClientFocusLost(current));
        }
    }

    actions
}

// Processing around map_request is currently copied from dwm:
//   - if override_redirect is set we completely ignore the window
//   - if the client is in the client_map (i.e. we are already managing this client) then ignore
fn process_map_request<'a, 'b, X>(
    state: WmState<'a, X>,
    id: Xid,
    override_redirect: bool,
) -> Vec<EventAction<'b>>
where
    X: XConn,
{
    if override_redirect || state.client_map.contains_key(&id) {
        vec![]
    } else {
        vec![EventAction::MapWindow(id)]
    }
}

fn process_property_notify<'a>(evt: PropertyEvent) -> Vec<EventAction<'a>> {
    match Atom::from_str(&evt.atom) {
        Ok(a) if a == Atom::WmName || a == Atom::NetWmName => {
            vec![EventAction::ClientNameChanged(evt.id, evt.is_root)]
        }
        _ => vec![],
        // TODO: handle other property changes and possibly allow users to process
        //       unknown events?
        // _ => vec![EventAction::UnknownPropertyChange(
        //     evt.id,
        //     evt.atom,
        //     evt.is_root,
        // )],
    }
}
