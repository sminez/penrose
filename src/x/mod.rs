use crate::{
    builtin::layout::messages::Hide,
    core::{
        bindings::{KeyCode, MouseState},
        ClientSet, Config, State,
    },
    pure::geometry::{Point, Rect},
    x::{atom::AUTO_FLOAT_WINDOW_TYPES, event::ClientMessage, property::WmState},
    Color, Result, Xid,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{error, trace};

pub mod atom;
pub mod event;
pub mod property;
pub mod query;

pub use atom::Atom;
pub use event::XEvent;
pub use property::{Prop, WindowAttributes};
pub use query::Query;

pub type ScreenId = usize;

/// A window type to be specified when creating a new window in the X server
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WinType {
    /// A simple hidden stub window for facilitating other API calls
    CheckWin,
    /// A window that receives input only (not queryable)
    InputOnly,
    /// A regular window. The [Atom] passed should be a
    /// valid _NET_WM_WINDOW_TYPE (this is not enforced)
    InputOutput(Atom),
}

/// On screen configuration options for X clients (not all are curently implemented)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ClientConfig {
    /// The border width in pixels
    BorderPx(u32),
    /// Absolute size and position on the screen as a [Rect]
    Position(Rect),
    /// Mark this window as stacking below the given Xid
    StackBelow(Xid),
    /// Mark this window as stacking on top of its peers
    StackAbove(Xid),
}

/// Attributes for an X11 client window (not all are curently implemented)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientAttr {
    /// Border color as an argb hex value
    BorderColor(u32),
    /// Set the pre-defined client event mask
    ClientEventMask,
    /// Set the pre-defined client event mask for sending unmap notify events
    ClientUnmapMask,
    /// Set the pre-defined root event mask
    RootEventMask,
}

pub trait XConn {
    fn root(&self) -> Xid;
    fn screen_details(&self) -> Result<Vec<Rect>>;
    fn cursor_position(&self) -> Result<Point>;

    fn grab(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()>;
    fn next_event(&self) -> Result<XEvent>;
    fn flush(&self);

    fn intern_atom(&self, atom: &str) -> Result<Xid>;
    fn atom_name(&self, xid: Xid) -> Result<String>;

    fn client_geometry(&self, client: Xid) -> Result<Rect>;
    fn existing_clients(&self) -> Result<Vec<Xid>>;

    fn map(&self, client: Xid) -> Result<()>;
    fn unmap(&self, client: Xid) -> Result<()>;
    fn kill(&self, client: Xid) -> Result<()>;
    fn focus(&self, client: Xid) -> Result<()>;

    fn get_prop(&self, client: Xid, prop_name: &str) -> Result<Option<Prop>>;
    fn get_window_attributes(&self, client: Xid) -> Result<WindowAttributes>;

    fn set_wm_state(&self, client: Xid, wm_state: WmState) -> Result<()>;
    fn set_prop(&self, client: Xid, name: &str, val: Prop) -> Result<()>;
    fn set_client_attributes(&self, client: Xid, attrs: &[ClientAttr]) -> Result<()>;
    fn set_client_config(&self, client: Xid, data: &[ClientConfig]) -> Result<()>;
    fn send_client_message(&self, msg: ClientMessage) -> Result<()>;

    fn warp_pointer(&self, id: Xid, x: i16, y: i16) -> Result<()>;
}

// Derivable methods for XConn that should never be given a different implementation
pub trait XConnExt: XConn + Sized {
    /// Kill the focused client if there is one
    fn kill_focused(&self, state: &mut State<Self>) -> Result<()> {
        if let Some(&id) = state.client_set.current_client() {
            self.kill(id)?;
        }

        Ok(())
    }

    fn manage(&self, id: Xid, state: &mut State<Self>) -> Result<()> {
        trace!(%id, "managing new client");
        manage_without_refresh(id, None, state, self)?;
        self.refresh(state)
    }

    fn unmanage(&self, client: Xid, state: &mut State<Self>) -> Result<()> {
        trace!(?client, "removing client");
        self.modify_and_refresh(state, |cs| {
            cs.remove_client(&client);
        })
    }

    /// Display a client on the screen by mapping it and setting its WmState to Normal
    /// This is idempotent if the client is already visible.
    fn reveal(&self, client: Xid, cs: &ClientSet, mapped: &mut HashSet<Xid>) -> Result<()> {
        self.set_wm_state(client, WmState::Normal)?;
        self.map(client)?;
        if cs.contains(&client) {
            mapped.insert(client);
        }

        Ok(())
    }

    /// Hide a client by unmapping it and setting its WmState to Iconic
    fn hide(
        &self,
        client: Xid,
        mapped: &mut HashSet<Xid>,
        pending_unmap: &mut HashMap<Xid, usize>,
    ) -> Result<()> {
        if !mapped.contains(&client) {
            return Ok(());
        }

        self.set_client_attributes(client, &[ClientAttr::ClientUnmapMask])?;
        self.unmap(client)?;
        self.set_client_attributes(client, &[ClientAttr::ClientEventMask])?;
        self.set_wm_state(client, WmState::Iconic)?;

        mapped.remove(&client);
        pending_unmap
            .entry(client)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        Ok(())
    }

    /// Apply a pure function that modifies a [ClientSet] and then handle refreshing the
    /// WindowManager state and associated X11 calls.
    ///
    /// This is the main logic that drives what the user will see on the screen in terms
    /// of window placement, focus and borders. Everything is driven from a diff of the
    /// pure ClientSet state before and after some mutating operation that was carried out
    /// by `f`.
    fn modify_and_refresh<F>(&self, state: &mut State<Self>, mut f: F) -> Result<()>
    where
        F: FnMut(&mut ClientSet),
    {
        f(&mut state.client_set); // NOTE: mutating the existing state

        let ss = state.client_set.position_and_snapshot();
        state.diff.update(ss);

        set_window_props(self, state)?;
        notify_hidden_workspaces(state);
        self.position_clients(&state.diff.after.positions)?;
        set_window_visibility(self, state)?;
        set_focus(self, state)?;
        handle_pointer_change(self, state)?;

        // TODO: clear enterWindow events from the event queue if this was because of mouse focus (?)

        let mut hook = state.config.refresh_hook.take();
        if let Some(ref mut h) = hook {
            trace!("running user refresh hook");
            if let Err(e) = h.call(state, self) {
                error!(%e, "error returned from user refresh hook");
            }
        }
        state.config.refresh_hook = hook;

        Ok(())
    }

    fn refresh(&self, state: &mut State<Self>) -> Result<()> {
        self.modify_and_refresh(state, |_| ())
    }

    fn client_should_float(&self, client: Xid, floating_classes: &[String]) -> Result<bool> {
        trace!(%client, "fetching WmTransientFor prop");
        if let Some(prop) = self.get_prop(client, Atom::WmTransientFor.as_ref())? {
            trace!(?prop, "window is transient: setting to floating state");
            return Ok(true);
        }

        trace!(%client, "fetching WmClass prop");
        if let Some(Prop::UTF8String(strs)) = self.get_prop(client, Atom::WmClass.as_ref())? {
            if strs.iter().any(|c| floating_classes.contains(c)) {
                trace!(%client, ?floating_classes, "window has a floating class: setting to floating state");
                return Ok(true);
            }
        }

        let float_types: Vec<&str> = AUTO_FLOAT_WINDOW_TYPES.iter().map(|a| a.as_ref()).collect();

        trace!(%client, "fetching NetWmWindowType prop");
        let p = self.get_prop(client, Atom::NetWmWindowType.as_ref())?;
        let should_float = if let Some(Prop::Atom(atoms)) = p {
            atoms.iter().any(|a| float_types.contains(&a.as_ref()))
        } else {
            false
        };

        Ok(should_float)
    }

    fn set_client_border_color<C>(&self, id: Xid, color: C) -> Result<()>
    where
        C: Into<Color>,
    {
        let color = color.into();
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color.rgb_u32())])
    }

    fn set_initial_properties(&self, client: Xid, config: &Config<Self>) -> Result<()> {
        let Config {
            normal_border,
            border_width,
            ..
        } = config;

        let conf = &[ClientConfig::BorderPx(*border_width)];
        let attrs = &[
            ClientAttr::ClientEventMask,
            ClientAttr::BorderColor(normal_border.rgb_u32()),
        ];

        self.set_wm_state(client, WmState::Iconic)?;
        self.set_client_attributes(client, attrs)?;
        self.set_client_config(client, conf)
    }

    fn position_client(&self, client: Xid, mut r: Rect) -> Result<()> {
        let p = Atom::WmNormalHints.as_ref();
        if let Ok(Some(Prop::WmNormalHints(hints))) = self.get_prop(client, p) {
            trace!(%client, ?hints, "client has WmNormalHints: applying size hints");
            r = hints.apply_to(r);
        }

        trace!(%client, ?r, "positioning client");
        self.set_client_config(client, &[ClientConfig::Position(r)])
    }

    fn position_clients(&self, positions: &[(Xid, Rect)]) -> Result<()> {
        self.restack(positions.iter().map(|(id, _)| id))?;

        for &(c, r) in positions.iter() {
            self.position_client(c, r)?;
        }

        Ok(())
    }

    /// Restack the given windows in, each one above the last.
    fn restack<'a, I>(&self, mut ids: I) -> Result<()>
    where
        I: Iterator<Item = &'a Xid>,
    {
        let mut previous = match ids.next() {
            Some(id) => *id,
            None => return Ok(()), // nothing to stack
        };

        for &id in ids {
            self.set_client_config(id, &[ClientConfig::StackAbove(previous)])?;
            previous = id;
        }

        Ok(())
    }

    fn set_active_client(&self, client: Xid, state: &mut State<Self>) -> Result<()> {
        self.modify_and_refresh(state, |cs| cs.focus_client(&client))
    }

    fn warp_pointer_to_window(&self, id: Xid) -> Result<()> {
        let r = self.client_geometry(id)?;

        self.warp_pointer(id, r.w as i16 / 2, r.h as i16 / 2)
    }

    fn warp_pointer_to_screen(&self, state: &mut State<Self>, screen_index: usize) -> Result<()> {
        let maybe_screen = state.client_set.screens().find(|s| s.index == screen_index);

        let screen = match maybe_screen {
            Some(s) => s,
            None => return Ok(()), // Unknown screen
        };

        if let Some(id) = screen.workspace.focus() {
            return self.warp_pointer_to_window(*id);
        }

        let x = (screen.r.x + screen.r.w / 2) as i16;
        let y = (screen.r.y + screen.r.h / 2) as i16;

        self.warp_pointer(self.root(), x, y)
    }

    fn window_title(&self, id: Xid) -> Result<String> {
        match query::str_prop(Atom::WmName, id, self) {
            Ok(Some(mut strs)) => Ok(strs.remove(0)),
            _ => match query::str_prop(Atom::NetWmName, id, self)? {
                Some(mut strs) => Ok(strs.remove(0)),
                None => Ok("".to_owned()),
            },
        }
    }
}

// Auto impl XConnExt for all XConn impls
impl<T> XConnExt for T where T: XConn {}

// The main logic for inserting a new client into the StackSet without any refresh
// of the X state. In normal window manager operation, the `manage` method on XConnExt
// is always used: this is provided independently to support managing existing clients
// on startup.
pub(crate) fn manage_without_refresh<X: XConn>(
    id: Xid,
    tag: Option<&str>,
    state: &mut State<X>,
    x: &X,
) -> Result<()> {
    let should_float = x.client_should_float(id, &state.config.floating_classes)?;
    let r = x.client_geometry(id)?;

    match tag {
        Some(tag) => state.client_set.insert_as_focus_for(tag, id),
        None => state.client_set.insert(id),
    }

    if should_float {
        state.client_set.float_unchecked(id, r);
    }

    let mut hook = state.config.manage_hook.take();
    if let Some(ref mut h) = hook {
        trace!("running user manage hook");
        if let Err(e) = h.call(id, state, x) {
            error!(%e, "error returned from user manage hook");
        }
    }
    state.config.manage_hook = hook;

    Ok(())
}

fn set_window_props<X: XConn>(x: &X, state: &mut State<X>) -> Result<()> {
    for &c in state.diff.new_clients() {
        x.set_initial_properties(c, &state.config)?;
    }

    if let Some(focused) = state.diff.before.focused_client {
        x.set_client_border_color(focused, state.config.normal_border)?;
    }

    if let Some(&focused) = state.client_set.current_client() {
        trace!(?focused, "setting border for focused client");
        x.set_client_border_color(focused, state.config.focused_border)?;
    }

    Ok(())
}

fn notify_hidden_workspaces<X: XConn>(state: &mut State<X>) {
    let previous_visible_tags = state.diff.previous_visible_tags();

    state
        .client_set
        .hidden_workspaces_mut()
        .filter(|w| previous_visible_tags.contains(&w.tag.as_ref()))
        .for_each(|ws| ws.broadcast_message(Hide));
}

// Warp the cursor if this diff resulted in a focus change
fn handle_pointer_change<X: XConn>(x: &X, state: &mut State<X>) -> Result<()> {
    if !state.config.focus_follow_mouse {
        return Ok(());
    }

    trace!("checking if focus should change");
    if !matches!(state.current_event, Some(XEvent::Enter(_))) {
        if let Some(id) = state.diff.focused_client() {
            trace!("focused client changed");
            // If the current workspace has any floating windows then warping the
            // cursor is not guaranteed to land us in the target window, which in
            // turn can lead to unexpected focus issues.
            // We _could_ attempt to be a lot smarter here and re-examine the floating
            // positions looking for any overlap of free screen space to move the
            // cursor to but that then leads to a lot of additional complexity while
            // still not providing any guaranteed behaviour. The simplest thing to
            // do (that is also the easiest to reason about) is to just not move the
            // cursor at all if there are any floating windows present.
            //
            // NOTE: Some of the behaviour here is based on looking at whether or
            //       not the focused client has changed position as part of this
            //       diff. That is going to cause issues if and when mouse based
            //       window movement is implemented.
            let tag = state.client_set.current_tag();
            let has_floating = state.client_set.has_floating_windows(tag);
            let focus_changed = state.diff.focused_client_changed();
            let focused_client_moved = state.diff.client_changed_position(&id);

            if !has_floating && (focus_changed || focused_client_moved) {
                trace!(
                    focus_changed,
                    focused_client_moved,
                    "warping to focused client"
                );
                x.warp_pointer_to_window(id)?;
            }
        } else if let Some(index) = state.diff.newly_focused_screen() {
            trace!(index, "screen changed: warping to screen");
            x.warp_pointer_to_screen(state, index)?;
        }
    }

    Ok(())
}

fn set_window_visibility<X: XConn>(x: &X, state: &mut State<X>) -> Result<()> {
    for &c in state.diff.visible_clients() {
        trace!(?c, "revealing client");
        x.reveal(c, &state.client_set, &mut state.mapped)?;
    }

    for &c in state.diff.hidden_clients() {
        trace!(?c, "hiding client");
        x.hide(c, &mut state.mapped, &mut state.pending_unmap)?;
    }

    for &c in state.diff.withdrawn_clients() {
        trace!(?c, "setting withdrawn state for client");
        x.set_wm_state(c, WmState::Withdrawn)?;
    }

    Ok(())
}

fn set_focus<X: XConn>(x: &X, state: &mut State<X>) -> Result<()> {
    if let Some(&id) = state.client_set.current_client() {
        x.focus(id)
    } else {
        x.focus(state.root)
    }
}
