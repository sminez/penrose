//! Logic for interacting with the X server
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
use tracing::{debug, error, trace};

pub mod atom;
pub mod event;
pub mod property;
pub mod query;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub use mock::{MockXConn, StubXConn};

pub use atom::Atom;
pub use event::XEvent;
pub use property::{Prop, WindowAttributes};
pub use query::Query;

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
    /// Mark this window as stacking on top of its peer
    StackAbove(Xid),
    /// Mark this window as stacking above all other windows
    StackTop,
    /// Mark this window as stacking below all other windows
    StackBottom,
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

/// A handle on a running X11 connection that we can use for issuing X requests.
///
/// XConn is intended as an abstraction layer to allow for communication with the underlying
/// display system (assumed to be X) using whatever mechanism the implementer wishes. In theory, it
/// should be possible to write an implementation that allows penrose to run on systems not using X
/// as the windowing system but X idioms and high level event types / client interations are
/// assumed.
pub trait XConn {
    /// The ID of the window manager root window.
    fn root(&self) -> Xid;
    /// Ask the X server for the dimensions of each currently available screen.
    fn screen_details(&self) -> Result<Vec<Rect>>;
    /// Ask the X server for the current (x, y) coordinate of the mouse cursor.
    fn cursor_position(&self) -> Result<Point>;

    /// Grab the specified key and mouse states, intercepting them for processing within
    /// the window manager itself.
    fn grab(&self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()>;
    /// Block and wait for the next event from the X server so it can be processed.
    fn next_event(&self) -> Result<XEvent>;
    /// Flush any pending events to the X server.
    fn flush(&self);

    /// Look up the [Xid] of a given [Atom] name. If it is not currently interned, intern it.
    fn intern_atom(&self, atom: &str) -> Result<Xid>;
    /// Look up the string name of a given [Atom] by its [Xid].
    fn atom_name(&self, xid: Xid) -> Result<String>;

    /// Look up the current dimensions and position of a given client window.
    fn client_geometry(&self, client: Xid) -> Result<Rect>;
    /// Ask the X server for the IDs of all currently known client windows
    fn existing_clients(&self) -> Result<Vec<Xid>>;

    /// Map the given client window to the screen with its current geometry, making it visible.
    fn map(&self, client: Xid) -> Result<()>;
    /// Unmap the given client window from the screen, hiding it.
    fn unmap(&self, client: Xid) -> Result<()>;
    /// Kill the given client window, closing it.
    fn kill(&self, client: Xid) -> Result<()>;
    /// Set X input focus to be held by the given client window.
    fn focus(&self, client: Xid) -> Result<()>;

    /// Look up a specific property on a given client window.
    fn get_prop(&self, client: Xid, prop_name: &str) -> Result<Option<Prop>>;
    /// List the known property names set for a given client.
    fn list_props(&self, client: Xid) -> Result<Vec<String>>;
    /// Get the current [WmState] for a given client window.
    fn get_wm_state(&self, client: Xid) -> Result<Option<WmState>>;
    /// Request the [WindowAttributes] for a given client window from the X server.
    fn get_window_attributes(&self, client: Xid) -> Result<WindowAttributes>;

    /// Set the current [WmState] for a given client window.
    fn set_wm_state(&self, client: Xid, wm_state: WmState) -> Result<()>;
    /// Set a specific property on a given client window.
    fn set_prop(&self, client: Xid, name: &str, val: Prop) -> Result<()>;
    /// Delete a property for a given client window.
    fn delete_prop(&self, client: Xid, prop_name: &str) -> Result<()>;
    /// Set one or more [ClientAttr] for a given client window.
    fn set_client_attributes(&self, client: Xid, attrs: &[ClientAttr]) -> Result<()>;
    /// Set the [ClientConfig] for a given client window.
    fn set_client_config(&self, client: Xid, data: &[ClientConfig]) -> Result<()>;
    /// Send a [ClientMessage] to a given client.
    fn send_client_message(&self, msg: ClientMessage) -> Result<()>;

    /// Reposition the mouse cursor to the given (x, y) coordinates within the specified window.
    /// This method should not be called directly: use `warp_pointer_to_window` or `warp_pointer_to_screen`
    /// instead.
    fn warp_pointer(&self, id: Xid, x: i16, y: i16) -> Result<()>;
}

/// Extended functionality for [XConn] impls in order to run the window manager.
pub trait XConnExt: XConn + Sized {
    /// Kill the focused client if there is one
    fn kill_focused(&self, state: &mut State<Self>) -> Result<()> {
        if let Some(&id) = state.client_set.current_client() {
            self.kill(id)?;
        }

        Ok(())
    }
    /// Establish the window manager state for the given client window and refresh the
    /// current X state.
    fn manage(&self, id: Xid, state: &mut State<Self>) -> Result<()> {
        trace!(%id, "managing new client");
        manage_without_refresh(id, None, state, self)?;
        self.refresh(state)
    }

    /// Remove the window manager state for the given client window and refresh the
    /// current X state.
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
        f(&mut state.client_set); // mutating the existing state

        let ss = state.position_and_snapshot(self);
        state.diff.update(ss);

        notify_killed(self, state)?;
        set_window_props(self, state)?;
        notify_hidden_workspaces(state);
        self.position_clients(state)?;
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

    /// Refresh the current X server state based on a diff of the current state against the state
    /// when we last refreshed.
    fn refresh(&self, state: &mut State<Self>) -> Result<()> {
        self.modify_and_refresh(state, |_| ())
    }

    /// Check whether or not the given client should be assigned floating status or not.
    fn client_should_float(&self, client: Xid, floating_classes: &[String]) -> Result<bool> {
        trace!(%client, "fetching WmClass prop");
        if let Some(Prop::UTF8String(strs)) = self.get_prop(client, Atom::WmClass.as_ref())? {
            if strs.iter().any(|c| floating_classes.contains(c)) {
                debug!(%client, ?floating_classes, "window has a floating class: setting to floating state");
                return Ok(true);
            }
        }

        trace!(%client, "fetching NetWmWindowType prop");
        let window_types = self.get_prop(client, Atom::NetWmWindowType.as_ref())?;
        debug!(?window_types, "client window types");

        let float_types: Vec<&str> = AUTO_FLOAT_WINDOW_TYPES.iter().map(|a| a.as_ref()).collect();

        let should_float = match window_types {
            Some(Prop::Atom(atoms)) => atoms.iter().any(|a| float_types.contains(&a.as_ref())),
            _ => false,
        };

        Ok(should_float)
    }

    /// Update the border color of the given client window.
    fn set_client_border_color<C>(&self, id: Xid, color: C) -> Result<()>
    where
        C: Into<Color>,
    {
        let color = color.into();
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color.argb_u32())])
    }

    /// Set the initial window properties for a newly managed window.
    fn set_initial_properties(&self, client: Xid, config: &Config<Self>) -> Result<()> {
        let Config {
            normal_border,
            border_width,
            ..
        } = config;

        let conf = &[ClientConfig::BorderPx(*border_width)];
        let attrs = &[
            ClientAttr::ClientEventMask,
            ClientAttr::BorderColor(normal_border.argb_u32()),
        ];

        self.set_wm_state(client, WmState::Iconic)?;
        self.set_client_attributes(client, attrs)?;
        self.set_client_config(client, conf)
    }

    /// Update the geometry of a given client based on the given [Rect].
    fn position_client(&self, client: Xid, mut r: Rect) -> Result<()> {
        let p = Atom::WmNormalHints.as_ref();
        if let Ok(Some(Prop::WmNormalHints(hints))) = self.get_prop(client, p) {
            trace!(%client, ?hints, "client has WmNormalHints: applying size hints");
            r = hints.apply_to(r);
        }

        trace!(%client, ?r, "positioning client");
        self.set_client_config(client, &[ClientConfig::Position(r)])
    }

    /// Restack and set the geometry for an ordered list of client windows and their
    /// associated positions. The provided positions are shrunk by the current border
    /// size in order to position the windows correctly within the frame given by the
    /// border.
    ///
    /// See `restack` for details of stacking order is determined.
    fn position_clients(&self, state: &State<Self>) -> Result<()> {
        let border = state.config.border_width;
        let positions = &state.diff.after.positions;
        let screen_positions: Vec<_> = state.client_set.screens().map(|s| s.r).collect();

        self.restack(positions.iter().map(|(id, _)| id))?;

        for &(c, mut r) in positions.iter() {
            if !screen_positions.contains(&r) {
                r = r.shrink_in(border);
            }
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

    /// Update the currently focused client and refresh the X state.
    fn set_active_client(&self, client: Xid, state: &mut State<Self>) -> Result<()> {
        self.modify_and_refresh(state, |cs| cs.focus_client(&client))
    }

    /// Warp the mouse cursor to the center of the given client window.
    fn warp_pointer_to_window(&self, id: Xid) -> Result<()> {
        let r = self.client_geometry(id)?;

        self.warp_pointer(id, r.w as i16 / 2, r.h as i16 / 2)
    }

    /// Warp the mouse cursor to the center of the given screen.
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

    /// Fetch the value of all known properties for a given client window
    fn all_props_for(&self, id: Xid) -> Result<HashMap<String, Prop>> {
        self.list_props(id)?
            .into_iter()
            .map(|s| {
                self.get_prop(id, &s)
                    .map(|opt| (s, opt.expect("prop to be set")))
            })
            .collect()
    }

    /// Request the title of a given client window following ICCCM/EWMH standards.
    fn window_title(&self, id: Xid) -> Result<String> {
        match query::str_prop(Atom::WmName, id, self) {
            Ok(Some(mut strs)) => Ok(strs.remove(0)),
            _ => match query::str_prop(Atom::NetWmName, id, self)? {
                Some(mut strs) => Ok(strs.remove(0)),
                None => Ok("".to_owned()),
            },
        }
    }

    /// Check to see if a given client window supports a particular protocol or not
    fn client_supports_protocol(&self, id: Xid, proto: &str) -> Result<bool> {
        if let Some(Prop::Atom(protocols)) = self.get_prop(id, Atom::WmProtocols.as_ref())? {
            Ok(protocols.iter().any(|p| p == proto))
        } else {
            Ok(false)
        }
    }

    /// Request a window's PID via the _NET_WM_PID property.
    ///
    /// **NOTE**: Not all programs set this property.
    fn window_pid(&self, id: Xid) -> Option<u32> {
        if let Ok(Some(Prop::Cardinal(vals))) = self.get_prop(id, "_NET_WM_PID") {
            Some(vals[0])
        } else {
            None
        }
    }

    /// Run the provided [Query], returning the result.
    fn query(&self, query: &dyn Query<Self>, id: Xid) -> Result<bool> {
        query.run(id, self)
    }

    /// Run the provided [Query], returning the result or a default value if there
    /// were any errors encountered when communicating with the X server.
    fn query_or(&self, default: bool, query: &dyn Query<Self>, id: Xid) -> bool {
        query.run(id, self).unwrap_or(default)
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
    trace!(%id, "fetching WmTransientFor prop");
    let (owned_tag, transient_for) = match x.get_prop(id, Atom::WmTransientFor.as_ref())? {
        Some(Prop::Window(ids)) => {
            let parent = ids[0];
            let owned_tag = state
                .client_set
                .tag_for_client(&parent)
                .or(tag)
                .map(|t| t.to_string());

            debug!(%id, %parent, ?owned_tag, "client is transient");
            (owned_tag, Some(parent))
        }

        _ => (tag.map(|t| t.to_string()), None),
    };

    let should_float =
        transient_for.is_some() || x.client_should_float(id, &state.config.floating_classes)?;

    match owned_tag {
        Some(tag) => state.client_set.insert_as_focus_for(tag.as_ref(), id),
        None => state.client_set.insert(id),
    }

    if should_float {
        debug!(%id, "client should float");
        let r = floating_client_position(id, transient_for, state, x)?;
        if state.client_set.float(id, r).is_err() {
            error!(%id, "attempted to float client which was not in state");
        }
    }

    let mut hook = state.config.manage_hook.take();
    if let Some(ref mut h) = hook {
        trace!("running user manage hook");
        if let Err(e) = h.call(id, state, x) {
            error!(%e, "error returned from user manage hook");
        }
    }
    state.config.manage_hook = hook;

    debug!(
        floating=?state.client_set.floating, "floating clients"
    );

    Ok(())
}

/// When positioning a floating client we try to position them in priority order of:
///   - the client's requested position if it is not at the origin
///   - centered in their parent's screen (if transient)
///   - centered in the focused screen
fn floating_client_position<X: XConn>(
    id: Xid,
    transient_for: Option<Xid>,
    state: &State<X>,
    x: &X,
) -> Result<Rect> {
    trace!(%id, "fetching client geometry");
    let r_initial = x.client_geometry(id)?;
    debug!(?r_initial, "initial geometry");

    if (r_initial.x, r_initial.y) != (0, 0) {
        debug!(?r_initial, "accepting client's requested position");
        return Ok(r_initial);
    }

    let r_parent = transient_for
        .and_then(|parent| state.client_set.screen_for_client(&parent))
        .unwrap_or(&state.client_set.screens.focus)
        .r;
    debug!(?r_parent, "parent geometry");

    let r_final = r_initial.centered_in(&r_parent).unwrap_or_else(|| {
        r_initial
            .centered_in(&state.client_set.screens.focus.r)
            .unwrap_or(r_initial)
    });
    debug!(?r_final, "final geometry");

    Ok(r_final)
}

fn notify_killed<X: XConn>(x: &X, state: &mut State<X>) -> Result<()> {
    for &c in state.diff.killed_clients() {
        x.kill(c)?;
    }

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
            // NOTE: Some of the behaviour here is based on looking at whether or
            //       not the focused client has changed position as part of this
            //       diff. That is going to cause issues if and when mouse based
            //       window movement is implemented.
            let focus_changed = state.diff.focused_client_changed();
            let focused_client_moved = state.diff.client_changed_position(&id);

            if focus_changed || focused_client_moved {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map, Error, Result};
    use simple_test_case::test_case;
    use std::collections::HashMap;

    #[derive(Default)]
    struct TransientXConn {
        transient_ids: HashMap<Xid, Xid>,
        geometry: HashMap<Xid, Rect>,
    }

    const TEST_SCREEN: Rect = Rect::new(0, 0, 1024, 768);
    const TEST_SCREEN_2: Rect = Rect::new(1024, 0, 4096, 2160);

    impl MockXConn for TransientXConn {
        fn mock_screen_details(&self) -> Result<Vec<Rect>> {
            Ok(vec![TEST_SCREEN, TEST_SCREEN_2])
        }

        fn mock_get_prop(&self, client: Xid, prop_name: &str) -> Result<Option<Prop>> {
            let maybe_prop = if prop_name == Atom::WmTransientFor.as_ref() {
                self.transient_ids
                    .get(&client)
                    .map(|id| Prop::Window(vec![*id]))
            } else {
                None
            };

            Ok(maybe_prop)
        }

        fn mock_client_geometry(&self, client: Xid) -> Result<Rect> {
            self.geometry
                .get(&client)
                .copied()
                .ok_or(Error::UnknownClient(client))
        }
    }

    #[test_case(
        Rect::new(0, 0, 600, 400),
        Rect::new(0, 0, 20, 20),
        0,
        Rect::new(502, 374, 20, 20);
        "fit inside parent"
    )]
    #[test_case(
        Rect::new(0, 0, 100, 200),
        Rect::new(0, 0, 200, 200),
        0,
        Rect::new(412, 284, 200, 200);
        "larger than parent"
    )]
    #[test_case(
        Rect::new(0, 0, 100, 200),
        Rect::new(0, 0, 2000, 2000),
        1,
        Rect::new(2072, 80, 2000, 2000);
        "larger than parent screen"
    )]
    #[test]
    fn manage_without_refresh_transient(parent: Rect, child: Rect, screen: usize, expected: Rect) {
        let conn = TransientXConn {
            transient_ids: map! {
                Xid(1) => Xid(2),
            },
            geometry: map! {
                Xid(1) => child,
                Xid(2) => parent,
            },
        };
        let mut state = State::try_new(Default::default(), &conn).expect("test state");
        state.client_set.focus_screen(screen);
        state.client_set.insert(Xid(2));
        state.client_set.focus_screen(0);

        manage_without_refresh(Xid(1), None, &mut state, &conn).expect("refresh");

        assert!(
            state.client_set.contains(&Xid(1)),
            "state contains managed transient"
        );

        let rel_rect = state.client_set.floating.get(&Xid(1));
        assert!(rel_rect.is_some(), "transient client is floating");

        let r_screen = [TEST_SCREEN, TEST_SCREEN_2][screen];
        let r = rel_rect.unwrap().applied_to(&r_screen);

        assert_eq!(r, expected, "client position is as expected");
    }
}
