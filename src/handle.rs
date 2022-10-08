//! XEvent handlers for use in the main event loop;
use crate::{
    bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
    core::{State, Xid},
    geometry::Point,
    x::{
        atom::Atom,
        event::{ClientMessage, ClientMessageKind},
        property::{Prop, WmHints},
        XConn, XConnExt,
    },
    StackSet,
};
use std::{mem::take, str::FromStr};
use tracing::{error, trace};

// fn is_fullscreen<X>(data: &[u32], x: &X) -> bool
// where
//     X: XConnExt,
// {
//     data.iter()
//         .map(|&a| x.atom_name(Xid(a)))
//         .flatten()
//         .any(|s| s == Atom::NetWmStateFullscreen.as_ref())
// }

pub(crate) fn client_message<X>(msg: ClientMessage, state: &mut State<X>, x: &X)
where
    X: XConn,
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

pub(crate) fn keypress<X>(key: KeyCode, bindings: &mut KeyBindings<X>, state: &mut State<X>)
where
    X: XConn,
{
    if let Some(action) = bindings.get_mut(&key) {
        if let Err(error) = action(state) {
            error!(%error, ?key, "error running user keybinding");
        }
    }
}

pub(crate) fn mouse_event<X>(e: MouseEvent, bindings: &mut MouseBindings<X>, state: &mut State<X>)
where
    X: XConn,
{
    if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
        if let Err(error) = action(state, &e) {
            error!(%error, ?e, "error running user mouse binding");
        }
    }
}

pub(crate) fn map_request<X>(client: Xid, state: &mut State<X>, x: &X)
where
    X: XConn,
{
    let attrs = x.get_window_attributes(client);

    if !state.client_set.contains(&client) && !attrs.override_redirect {
        x.manage(client, state);
    }
}

pub(crate) fn destroy<X>(client: Xid, state: &mut State<X>, x: &X)
where
    X: XConn,
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
pub(crate) fn unmap_notify<X>(client: Xid, state: &mut State<X>, x: &X)
where
    X: XConn,
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

pub(crate) fn focus_in<X>(client: Xid, state: &mut State<X>, x: &X)
where
    X: XConn,
{
    let accepts_focus = match x.get_prop(client, Atom::WmHints.as_ref()) {
        Some(Prop::WmHints(WmHints { accepts_input, .. })) => accepts_input,
        _ => true,
    };

    if accepts_focus {
        x.focus(client);
        x.set_prop(
            x.root(),
            Atom::NetActiveWindow.as_ref(),
            Prop::Window(vec![client]),
        );
        x.set_active_client(client, state);
    } else {
        let msg = ClientMessageKind::TakeFocus(client).as_message(x);
        x.send_client_message(msg);
    }
}

pub(crate) fn enter<X>(client: Xid, p: Point, state: &mut State<X>, x: &X)
where
    X: XConn,
{
    let focus_follow_mouse = state.config.focus_follow_mouse;

    x.modify_and_refresh(state, |cs| {
        if focus_follow_mouse {
            cs.focus_client(&client);
        }

        let maybe_tag = cs
            .iter_screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.workspace.tag.clone());

        if let Some(t) = maybe_tag {
            cs.focus_tag(&t);
        }
    });
}

pub(crate) fn leave<X>(client: Xid, p: Point, state: &mut State<X>, x: &X)
where
    X: XConn,
{
    if state.config.focus_follow_mouse {
        x.set_client_border_color(client, state.config.normal_border);
    }

    x.modify_and_refresh(state, |cs| {
        let maybe_tag = cs
            .iter_screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.workspace.tag.clone());

        if let Some(t) = maybe_tag {
            cs.focus_tag(&t);
        }
    });
}

pub(crate) fn detect_screens<X>(state: &mut State<X>, x: &X)
where
    X: XConn,
{
    let rects = x.screen_details();

    let StackSet {
        current,
        visible,
        hidden,
        floating,
    } = take(&mut state.client_set);

    let mut workspaces = vec![current.workspace];
    workspaces.extend(visible.into_iter().map(|s| s.workspace));
    workspaces.extend(hidden);

    // FIXME: this needs to not hard error. Probably best to pad with some default workspaces
    //        if there aren't enough already?
    state.client_set = StackSet::try_new_concrete(workspaces, rects, floating).unwrap();
}

pub(crate) fn screen_change<X>(state: &mut State<X>, x: &X)
where
    X: XConn,
{
    let p = x.cursor_position();

    x.modify_and_refresh(state, |cs| {
        let maybe_tag = cs
            .iter_screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.workspace.tag.clone());

        if let Some(t) = maybe_tag {
            cs.focus_tag(&t);
        }
    });
}
