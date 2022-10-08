//! XEvent handlers for use in the main event loop
use crate::{
    bindings::{KeyBindings, KeyCode},
    core::{State, Xid},
    x::{atom::Atom, event::ClientMessage, XConnExt},
};
use std::str::FromStr;
use tracing::{error, trace};

// match event {
//     // Direct 1-n mappings of XEvents -> EventActions
//     XEvent::Destroy(id) => vec![EventAction::DestroyClient(id)],
//     XEvent::Expose(_) => vec![], // FIXME: work out if this needs handling in the WindowManager
//     XEvent::FocusIn(id) => vec![EventAction::FocusIn(id)],
//     XEvent::KeyPress(code) => vec![EventAction::RunKeyBinding(code)],
//     XEvent::Leave(p) => vec![
//         EventAction::ClientFocusLost(p.id),
//         EventAction::SetScreenFromPoint(Some(p.abs)),
//     ],
//     XEvent::MouseEvent(evt) => vec![EventAction::RunMouseBinding(evt)],
//     XEvent::RandrNotify => vec![EventAction::DetectScreens],
//     XEvent::ScreenChange => vec![EventAction::SetScreenFromPoint(None)],
//     XEvent::UnmapNotify(id) => vec![EventAction::Unmap(id)],

//     // Require processing based on current WindowManager state
//     XEvent::ClientMessage(msg) => process_client_message(state, conn, msg),
//     XEvent::ConfigureNotify(evt) => process_configure_notify(evt),
//     XEvent::ConfigureRequest(evt) => process_configure_request(evt),
//     XEvent::Enter(p) => process_enter_notify(state, p),
//     XEvent::MapRequest(id, override_redirect) => {
//         process_map_request(state, id, override_redirect)
//     }
//     XEvent::PropertyNotify(evt) => process_property_notify(evt),
// }

// fn process_configure_notify(evt: ConfigureEvent) -> Vec<EventAction> {
//     if evt.is_root {
//         vec![EventAction::DetectScreens]
//     } else {
//         vec![]
//     }
// }

// fn process_configure_request(evt: ConfigureEvent) -> Vec<EventAction> {
//     if !evt.is_root {
//         vec![EventAction::MoveClientIfFloating(evt.id, evt.r)]
//     } else {
//         vec![]
//     }
// }

// fn process_enter_notify(state: &WmState, p: PointerChange) -> Vec<EventAction> {
//     let mut actions = vec![
//         EventAction::ClientFocusGained(p.id),
//         EventAction::SetScreenFromPoint(Some(p.abs)),
//     ];

//     if let Some(current) = state.clients.focused_client_id() {
//         if current != p.id {
//             actions.insert(0, EventAction::ClientFocusLost(current));
//         }
//     }

//     actions
// }

// fn process_property_notify(evt: PropertyEvent) -> Vec<EventAction> {
//     match Atom::from_str(&evt.atom) {
//         Ok(a) if a == Atom::WmName || a == Atom::NetWmName => {
//             vec![EventAction::ClientNameChanged(evt.id, evt.is_root)]
//         }
//         // TODO: handle other property changes and possibly allow users to process
//         //       unknown events?
//         _ => vec![EventAction::UnknownPropertyChange(
//             evt.id,
//             evt.atom,
//             evt.is_root,
//         )],
//     }
// }

// fn is_fullscreen<X>(data: &[u32], x: &X) -> bool
// where
//     X: XConnExt,
// {
//     data.iter()
//         .map(|&a| x.atom_name(Xid(a)))
//         .flatten()
//         .any(|s| s == Atom::NetWmStateFullscreen.as_ref())
// }

pub(crate) fn client_message<X>(msg: ClientMessage, state: &mut State, x: &X)
where
    X: XConnExt,
{
    let data = msg.data();
    trace!(id = msg.id.0, dtype = ?msg.dtype, ?data, "got client message");

    match Atom::from_str(&msg.dtype) {
        // Focus the requested window
        Ok(Atom::NetActiveWindow) => x.set_active_client(msg.id, state),

        // Focus the requested workspace by ID
        Ok(Atom::NetCurrentDesktop) => x.modify_and_refresh(state, |cs| {
            if let Some(t) = cs.tag_for_workspace_id(data.as_usize()[0]) {
                cs.focus_tag(&t);
            }
        }),

        // Move the target client to the requested workspace by ID
        Ok(Atom::NetWmDesktop) => x.modify_and_refresh(state, |cs| {
            if let Some(t) = cs.tag_for_workspace_id(data.as_usize()[0]) {
                cs.move_client_to_tag(&msg.id, &t);
            }
        }),

        // Toggle the requested client fullscreen
        // Ok(Atom::NetWmState) if is_fullscreen(&data.as_u32()[1..3], x) => {
        //     // _NET_WM_STATE_ADD == 1, _NET_WM_STATE_TOGGLE == 2
        //     x.set_fullscreen(msg.id, [1, 2].contains(&data.as_u32()[0]), state);
        // }

        // NOTE: all other client message types are ignored
        _ => (),
    }
}

pub(crate) fn keypress<X>(key: KeyCode, bindings: &mut KeyBindings, state: &mut State, _: &X)
where
    X: XConnExt,
{
    if let Some(action) = bindings.get_mut(&key) {
        if let Err(error) = action(state) {
            error!(%error, ?key, "error running user keybinding");
        }
    }
}

pub(crate) fn map_request<X>(client: Xid, state: &mut State, x: &X)
where
    X: XConnExt,
{
    let attrs = x.get_window_attributes(client);

    if !state.client_set.contains(&client) && !attrs.override_redirect {
        x.manage(client, state);
    }
}

pub(crate) fn destroy<X>(client: Xid, state: &mut State, x: &X)
where
    X: XConnExt,
{
    if state.client_set.contains(&client) {
        x.unmanage(client, state);
        state.mapped.remove(&client);
        state.pending_unmap.remove(&client);
    }

    // TODO: broadcast to layouts in case they need to know about this client being destroyed?
}

// Expected unmap events are tracked in pending_unmap. We ignore expected unmaps.
// FIXME: unmap notify events have a synthetic field I'm not currently checking?
//        that should be considered here as well apparently
pub(crate) fn unmap_notify<X>(client: Xid, state: &mut State, x: &X)
where
    X: XConnExt,
{
    let expected = *state.pending_unmap.get(&client).unwrap_or(&0);

    if expected == 0 {
        x.unmanage(client, state);
    } else if expected == 1 {
        state.pending_unmap.remove(&client);
    } else {
        state
            .pending_unmap
            .entry(client)
            .and_modify(|count| *count -= 1);
    }
}
