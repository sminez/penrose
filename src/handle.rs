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
    Result, StackSet,
};
use std::{mem::take, str::FromStr};
use tracing::{error, trace};

// fn is_fullscreen<X, E>(data: &[u32], x: &X) -> bool
// where
//     X: XConnExt,
// {
//     data.iter()
//         .map(|&a| x.atom_name(Xid(a)))
//         .flatten()
//         .any(|s| s == Atom::NetWmStateFullscreen.as_ref())
// }

pub(crate) fn client_message<X, E>(msg: ClientMessage, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
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
        _ => Ok(()),
    }
}

pub(crate) fn keypress<X, E>(
    key: KeyCode,
    bindings: &mut KeyBindings<X, E>,
    state: &mut State<X, E>,
    x: &X,
) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    if let Some(action) = bindings.get_mut(&key) {
        trace!(?key, "running user keybinding");
        if let Err(error) = action.call(state, x) {
            error!(%error, ?key, "error running user keybinding");
            return Err(error);
        }
    }

    Ok(())
}

pub(crate) fn mouse_event<X, E>(
    e: MouseEvent,
    bindings: &mut MouseBindings<X, E>,
    state: &mut State<X, E>,
    x: &X,
) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
        if let Err(error) = action.call(&e, state, x) {
            error!(%error, ?e, "error running user mouse binding");
            return Err(error);
        }
    }

    Ok(())
}

pub(crate) fn map_request<X, E>(client: Xid, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    trace!(?client, "handling new map request");
    let attrs = x.get_window_attributes(client)?;

    if !state.client_set.contains(&client) && !attrs.override_redirect {
        trace!(?client, "managing client");
        x.manage(client, state)?;
    }

    Ok(())
}

pub(crate) fn destroy<X, E>(client: Xid, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    if state.client_set.contains(&client) {
        trace!(?client, "destroying client");
        x.unmanage(client, state)?;
        state.mapped.remove(&client);
        state.pending_unmap.remove(&client);
    }

    // TODO: broadcast to layouts in case they need to know about this client being destroyed?

    Ok(())
}

// Expected unmap events are tracked in pending_unmap. We ignore expected unmaps.
// FIXME: unmap notify events have a synthetic field I'm not currently checking?
//        that should be considered here as well apparently
pub(crate) fn unmap_notify<X, E>(client: Xid, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    let expected = *state.pending_unmap.get(&client).unwrap_or(&0);

    if expected == 0 {
        x.unmanage(client, state)?;
    } else if expected == 1 {
        state.pending_unmap.remove(&client);
    } else {
        state
            .pending_unmap
            .entry(client)
            .and_modify(|count| *count -= 1);
    }

    Ok(())
}

pub(crate) fn focus_in<X, E>(client: Xid, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    let accepts_focus = match x.get_prop(client, Atom::WmHints.as_ref()) {
        Ok(Some(Prop::WmHints(WmHints { accepts_input, .. }))) => accepts_input,
        _ => true,
    };

    if accepts_focus {
        x.focus(client)?;
        x.set_prop(
            x.root(),
            Atom::NetActiveWindow.as_ref(),
            Prop::Window(vec![client]),
        )?;
        x.set_active_client(client, state)?;
    } else {
        let msg = ClientMessageKind::TakeFocus(client).as_message(x)?;
        x.send_client_message(msg)?;
    }

    Ok(())
}

pub(crate) fn enter<X, E>(client: Xid, p: Point, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
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
    })
}

pub(crate) fn leave<X, E>(client: Xid, p: Point, state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    if state.config.focus_follow_mouse {
        x.set_client_border_color(client, state.config.normal_border)?;
    }

    x.modify_and_refresh(state, |cs| {
        let maybe_tag = cs
            .iter_screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.workspace.tag.clone());

        if let Some(t) = maybe_tag {
            cs.focus_tag(&t);
        }
    })
}

pub(crate) fn detect_screens<X, E>(state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    let rects = x.screen_details()?;

    let StackSet {
        screens,
        hidden,
        floating,
        previous_tag,
    } = take(&mut state.client_set);

    let mut workspaces: Vec<_> = screens.into_iter().map(|s| s.workspace).collect();
    workspaces.extend(hidden);

    // FIXME: this needs to not hard error. Probably best to pad with some default workspaces
    //        if there aren't enough already?
    state.client_set = StackSet::try_new_concrete(workspaces, rects, floating)?;
    state.client_set.previous_tag = previous_tag;

    Ok(())
}

pub(crate) fn screen_change<X, E>(state: &mut State<X, E>, x: &X) -> Result<()>
where
    X: XConn,
    E: Send + Sync + 'static,
{
    let p = x.cursor_position()?;

    x.modify_and_refresh(state, |cs| {
        let maybe_tag = cs
            .iter_screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.workspace.tag.clone());

        if let Some(t) = maybe_tag {
            cs.focus_tag(&t);
        }
    })
}
