//! A platform agnostic backing connection
use crate::{
    Color, Result,
    builtin::layout::messages::Hide,
    core::{
        Config, State,
        bindings::{KeyBindings, MouseBindings, MouseState},
    },
    pure::{
        StackSet,
        geometry::{Point, Rect},
    },
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::{fmt, hash::Hash, ops::Deref};
use tracing::{debug, error, trace};

/// An ID for a given window
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct WinId(pub u32);

impl fmt::Display for WinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for WinId {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u32> for WinId {
    fn from(id: u32) -> Self {
        Self(id)
    }
}

impl From<WinId> for u32 {
    fn from(id: WinId) -> Self {
        id.0
    }
}

/// An event type associated with a [Conn]
pub trait ConnEvent:
    fmt::Debug + fmt::Display + Clone + PartialEq + Eq + Hash + Send + Sized
{
    /// Whether or not this event should trigger pointer warping as part of a refresh
    fn requires_pointer_warp(&self) -> bool;
}

/// A platform agnostic backing connection
pub trait Conn: Send + Sized {
    /// The event type used by this connection
    type Event: ConnEvent;

    /// Additional data that will be kept separately within the main [State].
    ///
    /// This is typically used to be able to support having generic shared state when writing
    /// families of Conn implementations such as `XConn`.
    type State: fmt::Debug + Send + Sized;

    /// The type that is used as the key for a [KeyBindings] map using this Conn.
    type KeyBindingKey: fmt::Debug + Copy + Clone + PartialEq + Eq + Hash + Send + Sized;

    /// Called once when the window manager state is first created.
    fn initial_state(&mut self) -> Self::State;

    /// The ID of the window manager root window.
    fn root(&mut self) -> WinId;
    /// Block and wait for the next event so it can be processed.
    fn next_event(&mut self) -> Result<Self::Event>;
    /// Process the an event
    fn handle_event(
        &mut self,
        evt: Self::Event,
        key_bindings: &mut KeyBindings<Self>,
        mouse_bindings: &mut MouseBindings<Self>,
        state: &mut State<Self>,
    ) -> Result<()>;
    /// Flush any pending events to the underlying back end.
    fn flush(&mut self);

    /// Capture the next non-modifier key press, bound or not, delivering it to the window
    /// manager rather than to the focused client. This is how a chorded binding knows to
    /// abort rather than wait indefinitely.
    ///
    /// One-shot: implementations must end the capture as soon as a key press is delivered, so
    /// that a caller which never cancels cannot hold the keyboard. Backends which do nothing
    /// here leave chords waiting rather than aborting.
    fn capture_next_key(&mut self) -> Result<()> {
        Ok(())
    }

    /// End a capture without waiting for the key press that would have ended it.
    ///
    /// A no-op when nothing is in flight, which is the usual case: a delivered key press ends
    /// its own capture. This is for abandoning one that nothing is going to satisfy.
    fn cancel_capture_next_key(&mut self) -> Result<()> {
        Ok(())
    }

    /// Grab the specified key and mouse states, intercepting them for processing within
    /// the window manager itself.
    ///
    /// This *replaces* the currently grabbed set: anything previously grabbed and not named
    /// here is released.
    fn grab(
        &mut self,
        key_codes: &[Self::KeyBindingKey],
        mouse_states: &[MouseState],
    ) -> Result<()>;
    /// Ask the X server for the IDs of all currently known client windows
    fn existing_clients(&mut self) -> Result<Vec<WinId>>;
    /// Request a client windows's current workspace
    fn manage_existing_clients(&mut self, state: &mut State<Self>) -> Result<()>;
    /// The dimensions of each currently available screen.
    fn screen_details(&mut self) -> Result<Vec<Rect>>;
    /// The current (x, y) coordinate of the mouse cursor.
    fn cursor_position(&mut self) -> Result<Point>;
    /// Reposition the mouse cursor to the given (x, y) coordinates within the specified window.
    fn warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()>;

    /// Update the geometry of a given client based on the given [Rect].
    fn position_client(&mut self, id: WinId, r: Rect) -> Result<()>;
    /// Display a client on the screen at its current position.
    fn show_client(&mut self, id: WinId, state: &mut State<Self>) -> Result<()>;
    /// Hide a client
    fn hide_client(&mut self, id: WinId, state: &mut State<Self>) -> Result<()>;
    /// Withdraw a client
    fn withdraw_client(&mut self, id: WinId) -> Result<()>;
    /// Kill the given client window, closing it.
    fn kill_client(&mut self, id: WinId) -> Result<()>;
    /// Set input focus to be held by the given client window.
    fn focus_client(&mut self, id: WinId) -> Result<()>;

    /// Look up the current dimensions and position of a given client window.
    fn client_geometry(&mut self, id: WinId) -> Result<Rect>;
    /// Request the title of a given client window.
    fn client_title(&mut self, id: WinId) -> Result<String>;
    /// Request a window's PID.
    fn client_pid(&mut self, id: WinId) -> Option<u32>;
    /// Check whether or not the given client should be assigned floating status or not.
    fn client_should_float(&mut self, id: WinId, floating_classes: &[String]) -> bool;
    /// For a given existing client being processed on startup, determine whether we need
    /// to bring it into our internal state and manage it.
    fn client_should_be_managed(&mut self, id: WinId) -> bool;
    /// Check whether this client is currently fullscreen or not
    fn client_is_fullscreen(&mut self, id: WinId) -> bool;
    /// The id of the parent for this client if it is transient
    fn client_transient_parent(&mut self, id: WinId) -> Option<WinId>;

    /// Update the border color of the given client window.
    fn set_client_border_color(&mut self, id: WinId, color: impl Into<Color>) -> Result<()>;
    /// Set the initial window properties for a newly managed window.
    fn set_initial_properties(&mut self, id: WinId, config: &Config<Self>) -> Result<()>;

    /// Restack the given windows, each one above the last.
    fn restack<'a, I>(&mut self, ids: I) -> Result<()>
    where
        WinId: 'a,
        I: Iterator<Item = &'a WinId>;
}

/// Extended functionality for [Conn] impls in order to run the window manager.
pub trait ConnExt: Conn + Sized {
    /// Kill the focused client if there is one
    fn kill_focused(&mut self, state: &mut State<Self>) -> Result<()> {
        if let Some(&id) = state.client_set.current_client() {
            self.kill_client(id)?;
        }

        Ok(())
    }

    /// Establish the window manager state for the given client window and refresh the
    /// current X state.
    fn manage(&mut self, id: WinId, state: &mut State<Self>) -> Result<()> {
        trace!(%id, "managing new client");
        manage_without_refresh(id, None, state, self)?;
        self.refresh(state)
    }

    /// Remove the window manager state for the given client window and refresh the
    /// current X state.
    fn unmanage(&mut self, id: WinId, state: &mut State<Self>) -> Result<()> {
        trace!(?id, "removing client");
        self.modify_and_refresh(state, |cs| {
            cs.remove_client(&id);
        })
    }

    /// Apply a pure function that modifies a [ClientSet] and then handle refreshing the
    /// WindowManager state and associated X11 calls.
    ///
    /// This is the main logic that drives what the user will see on the screen in terms
    /// of window placement, focus and borders. Everything is driven from a diff of the
    /// pure ClientSet state before and after some mutating operation that was carried out
    /// by `f`.
    fn modify_and_refresh<F>(&mut self, state: &mut State<Self>, mut f: F) -> Result<()>
    where
        F: FnMut(&mut StackSet<WinId>),
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
    fn refresh(&mut self, state: &mut State<Self>) -> Result<()> {
        self.modify_and_refresh(state, |_| ())
    }

    /// Restack and set the geometry for an ordered list of client windows and their
    /// associated positions. The provided positions are shrunk by the current border
    /// size in order to position the windows correctly within the frame given by the
    /// border.
    ///
    /// See `restack` for details of stacking order is determined.
    fn position_clients(&mut self, state: &State<Self>) -> Result<()> {
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

    /// Update the currently focused client and refresh the X state.
    fn set_active_client(&mut self, id: WinId, state: &mut State<Self>) -> Result<()> {
        self.modify_and_refresh(state, |cs| cs.focus_client(&id))
    }

    /// Warp the mouse cursor to the center of the given client window.
    fn warp_pointer_to_window(&mut self, id: WinId) -> Result<()> {
        let r = self.client_geometry(id)?;

        self.warp_pointer(id, r.w as i16 / 2, r.h as i16 / 2)
    }

    /// Warp the mouse cursor to the center of the given screen.
    fn warp_pointer_to_screen(
        &mut self,
        state: &mut State<Self>,
        screen_index: usize,
    ) -> Result<()> {
        let maybe_screen = state.client_set.screens().find(|s| s.index == screen_index);

        let screen = match maybe_screen {
            Some(s) => s,
            None => return Ok(()), // Unknown screen
        };

        if let Some(id) = screen.workspace.focus() {
            return self.warp_pointer_to_window(*id);
        }

        let x = (screen.r.x + screen.r.w as i32 / 2) as i16;
        let y = (screen.r.y + screen.r.h as i32 / 2) as i16;
        let root = self.root();

        self.warp_pointer(root, x, y)
    }

    /// Run the provided [Query], returning the result.
    fn query(&mut self, query: &dyn Query<Self>, id: WinId) -> Result<bool> {
        query.run(id, self)
    }

    /// Run the provided [Query], returning the result or a default value if there
    /// were any errors encountered when communicating with the X server.
    fn query_or(&mut self, default: bool, query: &dyn Query<Self>, id: WinId) -> bool {
        query.run(id, self).unwrap_or(default)
    }
}

// Auto impl XConnExt for all XConn impls
impl<T> ConnExt for T where T: Conn {}

/// A query to be run against client windows for identifying specific windows
/// or programs.
pub trait Query<C: Conn>: Send {
    /// Run this query for a given window ID.
    fn run(&self, id: WinId, conn: &mut C) -> Result<bool>;

    /// Combine this query with another query using a logical AND.
    ///
    /// This follows typical short-circuiting behavior, i.e. if the first query
    /// returns false, the second query will not be run.
    fn and<Other>(self, other: Other) -> AndQuery<C>
    where
        Self: Sized + 'static,
        Other: Query<C> + 'static,
    {
        AndQuery {
            first: Box::new(self),
            second: Box::new(other),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Combine this query with another query using a logical OR.
    ///
    /// This follows typical short-circuiting behavior, i.e. if the first query
    /// returns true, the second query will not be run.
    fn or<Other>(self, other: Other) -> OrQuery<C>
    where
        Self: Sized + 'static,
        Other: Query<C> + 'static,
    {
        OrQuery {
            first: Box::new(self),
            second: Box::new(other),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Apply a logical NOT to this query.
    ///
    /// This will invert the result of the query.
    fn not(self) -> NotQuery<C>
    where
        Self: Sized + 'static,
    {
        NotQuery {
            inner: Box::new(self),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<C: Conn> fmt::Debug for Box<dyn Query<C>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Query").finish()
    }
}

/// A meta [Query] for combining two queries with a logical AND.
#[derive(Debug)]
pub struct AndQuery<C: Conn> {
    first: Box<dyn Query<C>>,
    second: Box<dyn Query<C>>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Conn> Query<C> for AndQuery<C> {
    fn run(&self, id: WinId, x: &mut C) -> Result<bool> {
        Ok(self.first.run(id, x)? && self.second.run(id, x)?)
    }
}

/// A meta [Query] for combining two queries with a logical OR.
#[derive(Debug)]
pub struct OrQuery<C: Conn> {
    first: Box<dyn Query<C>>,
    second: Box<dyn Query<C>>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Conn> Query<C> for OrQuery<C> {
    fn run(&self, id: WinId, x: &mut C) -> Result<bool> {
        Ok(self.first.run(id, x)? || self.second.run(id, x)?)
    }
}

/// A meta [Query] for applying a logical NOT to a query.
#[derive(Debug)]
pub struct NotQuery<C: Conn> {
    inner: Box<dyn Query<C>>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Conn> Query<C> for NotQuery<C> {
    fn run(&self, id: WinId, x: &mut C) -> Result<bool> {
        Ok(!self.inner.run(id, x)?)
    }
}

/// The main logic for inserting a new client into the StackSet without any refresh
/// of the X state. In normal window manager operation, the `manage` method on XConnExt
/// is always used: this is provided independently to support managing existing clients
/// on startup.
pub fn manage_without_refresh<C: Conn>(
    id: WinId,
    tag: Option<&str>,
    state: &mut State<C>,
    conn: &mut C,
) -> Result<()> {
    trace!(%id, "checking if client is transient");
    let transient_for = conn.client_transient_parent(id);
    trace!(%id, "checking if client should float");
    let should_float = conn.client_should_float(id, &state.config.floating_classes);
    trace!(%id, "checking for owned tag");
    let owned_tag = transient_for
        .and_then(|parent| state.client_set.tag_for_client(&parent))
        .or(tag)
        .map(|t| t.to_string());

    trace!(%id, "inserting client");
    match owned_tag {
        Some(tag) => state.client_set.insert_as_focus_for(tag.as_ref(), id),
        None => state.client_set.insert(id),
    }

    if transient_for.is_some() || should_float {
        debug!(%id, "client should float");
        let r = floating_client_position(id, transient_for, state, conn)?;
        if state.client_set.float(id, r).is_err() {
            error!(%id, "attempted to float client which was not in state");
        }
    }

    let mut hook = state.config.manage_hook.take();
    if let Some(ref mut h) = hook {
        trace!("running user manage hook");
        if let Err(e) = h.call(id, state, conn) {
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
fn floating_client_position<C: Conn>(
    id: WinId,
    transient_for: Option<WinId>,
    state: &State<C>,
    conn: &mut C,
) -> Result<Rect> {
    trace!(%id, "fetching client geometry");
    let r_initial = conn.client_geometry(id)?;
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

fn notify_killed<C: Conn>(conn: &mut C, state: &mut State<C>) -> Result<()> {
    for c in state.diff.killed_clients() {
        conn.kill_client(c)?;
    }

    Ok(())
}

fn set_window_props<C: Conn>(conn: &mut C, state: &mut State<C>) -> Result<()> {
    for c in state.diff.new_clients() {
        conn.set_initial_properties(c, &state.config)?;
    }

    if let Some(focused) = state.diff.before.focused_client {
        conn.set_client_border_color(focused, state.config.normal_border)?;
    }

    if let Some(&focused) = state.client_set.current_client() {
        trace!(?focused, "setting border for focused client");
        conn.set_client_border_color(focused, state.config.focused_border)?;
    }

    Ok(())
}

fn notify_hidden_workspaces<C: Conn>(state: &mut State<C>) {
    let previous_visible_tags = state.diff.previous_visible_tags();

    state
        .client_set
        .hidden_workspaces_mut()
        .filter(|w| previous_visible_tags.contains(&w.tag.as_ref()))
        .for_each(|ws| ws.broadcast_message(Hide));
}

// Warp the cursor if this diff resulted in a focus change
fn handle_pointer_change<C: Conn>(conn: &mut C, state: &mut State<C>) -> Result<()> {
    if !state.config.focus_follow_mouse {
        return Ok(());
    }

    let require_pointer_warp = state.current_event().map(|e| e.requires_pointer_warp());
    trace!(?require_pointer_warp, "checking if focus should change");
    if require_pointer_warp == Some(true) {
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
                    focused_client_moved, "warping to focused client"
                );
                conn.warp_pointer_to_window(id)?;
            }
        } else if let Some(index) = state.diff.newly_focused_screen() {
            trace!(index, "screen changed: warping to screen");
            conn.warp_pointer_to_screen(state, index)?;
        }
    }

    Ok(())
}

fn set_window_visibility<C: Conn>(conn: &mut C, state: &mut State<C>) -> Result<()> {
    for c in state.diff.visible_clients() {
        trace!(?c, "revealing client");
        conn.show_client(c, state)?;
    }

    for c in state.diff.hidden_clients() {
        trace!(?c, "hiding client");
        conn.hide_client(c, state)?;
    }

    for c in state.diff.withdrawn_clients() {
        trace!(?c, "setting withdrawn state for client");
        conn.withdraw_client(c)?;
    }

    Ok(())
}

fn set_focus<C: Conn>(conn: &mut C, state: &mut State<C>) -> Result<()> {
    if let Some(&id) = state.client_set.current_client() {
        conn.focus_client(id)
    } else {
        conn.focus_client(state.root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Error, Result, map,
        x::{Atom, MockXConn, Prop},
    };
    use simple_test_case::test_case;
    use std::collections::HashMap;

    #[derive(Default)]
    struct TransientXConn {
        transient_ids: HashMap<WinId, WinId>,
        geometry: HashMap<WinId, Rect>,
    }

    const TEST_SCREEN: Rect = Rect::new(0, 0, 1024, 768);
    const TEST_SCREEN_2: Rect = Rect::new(1024, 0, 4096, 2160);

    impl MockXConn for TransientXConn {
        fn mock_screen_details(&mut self) -> Result<Vec<Rect>> {
            Ok(vec![TEST_SCREEN, TEST_SCREEN_2])
        }

        fn mock_get_prop(&mut self, client: WinId, prop_name: &str) -> Result<Option<Prop>> {
            let maybe_prop = if prop_name == Atom::WmTransientFor.as_ref() {
                self.transient_ids
                    .get(&client)
                    .map(|id| Prop::Window(vec![*id]))
            } else {
                None
            };

            Ok(maybe_prop)
        }

        fn mock_client_geometry(&mut self, client: WinId) -> Result<Rect> {
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
        let mut conn = TransientXConn {
            transient_ids: map! {
                WinId(1) => WinId(2),
            },
            geometry: map! {
                WinId(1) => child,
                WinId(2) => parent,
            },
        };
        let mut state = State::try_new(Default::default(), &mut conn).expect("test state");
        state.client_set.focus_screen(screen);
        state.client_set.insert(WinId(2));
        state.client_set.focus_screen(0);

        manage_without_refresh(WinId(1), None, &mut state, &mut conn).expect("refresh");

        assert!(
            state.client_set.contains(&WinId(1)),
            "state contains managed transient"
        );

        let rel_rect = state.client_set.floating.get(&WinId(1));
        assert!(rel_rect.is_some(), "transient client is floating");

        let r_screen = [TEST_SCREEN, TEST_SCREEN_2][screen];
        let r = rel_rect.unwrap().applied_to(&r_screen);

        assert_eq!(r, expected, "client position is as expected");
    }
}
