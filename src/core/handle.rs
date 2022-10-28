//! XEvent handlers for use in the main event loop;
use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
        State, Xid,
    },
    pure::{geometry::Point, Workspace},
    x::{
        atom::Atom,
        event::{ClientMessage, ClientMessageKind, PointerChange},
        property::{Prop, WmHints},
        XConn, XConnExt,
    },
    Result, StackSet,
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

pub(crate) fn client_message<X>(msg: ClientMessage, state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
{
    let data = &msg.data;
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

pub(crate) fn keypress<X>(
    key: KeyCode,
    bindings: &mut KeyBindings<X>,
    state: &mut State<X>,
    x: &X,
) -> Result<()>
where
    X: XConn,
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

pub(crate) fn mouse_event<X>(
    e: MouseEvent,
    bindings: &mut MouseBindings<X>,
    state: &mut State<X>,
    x: &X,
) -> Result<()>
where
    X: XConn,
{
    if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
        if let Err(error) = action.call(&e, state, x) {
            error!(%error, ?e, "error running user mouse binding");
            return Err(error);
        }
    }

    Ok(())
}

pub(crate) fn map_request<X>(client: Xid, state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
{
    trace!(?client, "handling new map request");
    let attrs = x.get_window_attributes(client)?;

    if !state.client_set.contains(&client) && !attrs.override_redirect {
        trace!(?client, "managing client");
        x.manage(client, state)?;
    }

    Ok(())
}

pub(crate) fn destroy<X>(client: Xid, state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
{
    if state.client_set.contains(&client) {
        trace!(?client, "destroying client");
        x.unmanage(client, state)?;
        state.mapped.remove(&client);
        state.pending_unmap.remove(&client);
    }

    Ok(())
}

// Expected unmap events are tracked in pending_unmap. We ignore expected unmaps.
pub(crate) fn unmap_notify<X>(client: Xid, state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
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

pub(crate) fn focus_in<X>(client: Xid, state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
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

pub(crate) fn enter<X>(
    PointerChange { id, .. }: PointerChange,
    state: &mut State<X>,
    x: &X,
) -> Result<()>
where
    X: XConn,
{
    if state.config.focus_follow_mouse {
        x.modify_and_refresh(state, |cs| {
            cs.focus_client(&id);
        })
    } else {
        Ok(())
    }
}

pub(crate) fn leave<X>(
    PointerChange {
        id, same_screen, ..
    }: PointerChange,
    state: &mut State<X>,
    x: &X,
) -> Result<()>
where
    X: XConn,
{
    if id == state.root() && !same_screen {
        x.focus(id)?;
    }

    Ok(())
}

pub(crate) fn detect_screens<X>(state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
{
    let rects = x.screen_details()?;

    let StackSet {
        screens,
        hidden,
        floating,
        previous_tag,
        invisible_tags,
    } = take(&mut state.client_set);

    let mut workspaces: Vec<_> = screens.into_iter().map(|s| s.workspace).collect();
    workspaces.extend(hidden);

    // Pad out the workspace list with default workspaces if there aren't enough available
    // to cover the attached screens.
    // NOTE: This can still error if we end up with a tag collision because the user has
    //       named one of there tags with the one we generate based on ID.
    if workspaces.len() < rects.len() {
        let n_short = rects.len() - workspaces.len();
        let next_id = workspaces.iter().map(|w| w.id).max().unwrap_or(0) + 1;
        workspaces.extend((0..n_short).map(|n| Workspace::new_default(n + next_id)))
    }

    state.client_set = StackSet {
        previous_tag,
        invisible_tags,
        ..StackSet::try_new_concrete(workspaces, rects, floating)?
    };

    Ok(())
}

pub(crate) fn screen_change<X>(state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
{
    set_screen_from_point(x.cursor_position()?, state, x)
}

fn set_screen_from_point<X>(p: Point, state: &mut State<X>, x: &X) -> Result<()>
where
    X: XConn,
{
    x.modify_and_refresh(state, |cs| {
        let index = cs
            .iter_screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.index());

        if let Some(index) = index {
            cs.focus_screen(index);
        }
    })
}
