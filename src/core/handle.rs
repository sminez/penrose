//! XEvent handlers for use in the main event loop;
use crate::{
    core::{
        bindings::{
            KeyBindings, KeyCode, MotionNotifyEvent, MouseBindings, MouseEvent, MouseEventKind,
        },
        conn::{Conn, ConnExt, WinId},
        State,
    },
    pure::geometry::Point,
    x::event::{ClientMessage, ConfigureEvent, PointerChange},
    Result,
};
use tracing::{error, info, trace};

// Currently no client messages are handled by default (see the ewmh extension for some examples of messages
// that are handled when that is enabled)
pub(crate) fn client_message<C: Conn>(msg: ClientMessage, _: &mut State<C>, _: &C) -> Result<()> {
    let data = &msg.data;
    trace!(id = msg.id.0, dtype = ?msg.dtype, ?data, "got client message");

    Ok(())
}

pub(crate) fn mapping_notify<C: Conn>(
    key_bindings: &KeyBindings<C>,
    mouse_bindings: &MouseBindings<C>,
    conn: &C,
) -> Result<()> {
    trace!("grabbing key and mouse bindings");
    let key_codes: Vec<_> = key_bindings.keys().copied().collect();
    let mouse_states: Vec<_> = mouse_bindings.keys().cloned().collect();

    conn.grab(&key_codes, &mouse_states)
}

pub(crate) fn keypress<C: Conn>(
    key: KeyCode,
    bindings: &mut KeyBindings<C>,
    state: &mut State<C>,
    conn: &C,
) -> Result<()> {
    if let Some(action) = bindings.get_mut(&key) {
        trace!(?key, "running user keybinding");
        if let Err(error) = action.call(state, conn) {
            error!(%error, ?key, "error running user keybinding");
            return Err(error);
        }
    }

    Ok(())
}

pub(crate) fn mouse_event<C: Conn>(
    e: MouseEvent,
    bindings: &mut MouseBindings<C>,
    state: &mut State<C>,
    conn: &C,
) -> Result<()> {
    if let Some(action) = bindings.get_mut(&e.state) {
        if let Err(error) = action.on_mouse_event(&e, state, conn) {
            error!(%error, ?e, "error running user mouse binding");
            return Err(error);
        }

        match e.kind {
            MouseEventKind::Press => state.held_mouse_state = Some(e.state),
            MouseEventKind::Release => state.held_mouse_state = None,
        }
    }

    Ok(())
}

pub(crate) fn motion_event<C: Conn>(
    e: MotionNotifyEvent,
    bindings: &mut MouseBindings<C>,
    state: &mut State<C>,
    conn: &C,
) -> Result<()> {
    let held_state = match state.held_mouse_state.as_ref() {
        Some(state) => state,
        None => return Ok(()), // motion without us holding anything
    };

    if let Some(action) = bindings.get_mut(held_state) {
        if let Err(error) = action.on_motion(&e, state, conn) {
            error!(%error, ?e, "error running user mouse binding");
            return Err(error);
        }
    }

    Ok(())
}

pub(crate) fn configure_request<C: Conn>(
    ConfigureEvent { id, r, .. }: &ConfigureEvent,
    state: &mut State<C>,
    conn: &C,
) -> Result<()> {
    if state.client_set.contains(id) && !state.client_set.floating.contains_key(id) {
        return Ok(()); // Managed tiled clients aren't allowed to configure themselves
    }

    conn.position_client(*id, *r)
}

pub(crate) fn map_request<C: Conn>(id: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
    trace!(?id, "handling new map request");

    if !state.client_set.contains(&id) && conn.client_should_be_managed(id) {
        trace!(?id, "managing client");
        conn.manage(id, state)?;
    }

    Ok(())
}

pub(crate) fn destroy<C: Conn>(id: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
    trace!(?id, "destroying client");
    conn.unmanage(id, state)?;
    state.mapped.remove(&id);
    state.pending_unmap.remove(&id);

    Ok(())
}

// Expected unmap events are tracked in pending_unmap. We ignore expected unmaps.
pub(crate) fn unmap_notify<C: Conn>(id: WinId, state: &mut State<C>, conn: &C) -> Result<()> {
    let expected = *state.pending_unmap.get(&id).unwrap_or(&0);

    if expected == 0 {
        conn.unmanage(id, state)?;
    } else if expected == 1 {
        state.pending_unmap.remove(&id);
    } else {
        state
            .pending_unmap
            .entry(id)
            .and_modify(|count| *count -= 1);
    }

    Ok(())
}

pub(crate) fn enter<C: Conn>(p: PointerChange, state: &mut State<C>, conn: &C) -> Result<()> {
    if state.config.focus_follow_mouse {
        conn.modify_and_refresh(state, |cs| {
            cs.focus_client(&p.id);
        })
    } else {
        Ok(())
    }
}

pub(crate) fn leave<C: Conn>(p: PointerChange, state: &mut State<C>, conn: &C) -> Result<()> {
    if p.id == state.root() && !p.same_screen {
        conn.focus_client(p.id)?;
        set_screen_from_point(p.abs, state, conn)?;
    }

    Ok(())
}

pub(crate) fn detect_screens<C: Conn>(state: &mut State<C>, conn: &C) -> Result<()> {
    info!("re-detecting screens");
    let rects = conn.screen_details()?;
    info!(?rects, "found screens");

    state.client_set.update_screens(rects)
}

pub(crate) fn screen_change<C: Conn>(state: &mut State<C>, conn: &C) -> Result<()> {
    trace!("screen changed");
    set_screen_from_point(conn.cursor_position()?, state, conn)
}

fn set_screen_from_point<C: Conn>(p: Point, state: &mut State<C>, conn: &C) -> Result<()> {
    conn.modify_and_refresh(state, |cs| {
        let index = cs
            .screens()
            .find(|s| s.r.contains_point(p))
            .map(|s| s.index());

        if let Some(index) = index {
            cs.focus_screen(index);
        }
    })
}
