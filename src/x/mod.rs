//! Logic for interacting with the X server
use crate::{
    Color, Result, WinId,
    core::{
        Config, State,
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseState},
        conn::{Conn, ConnEvent, ConnExt, manage_without_refresh},
    },
    pure::geometry::{Point, Rect},
    x::{
        atom::AUTO_FLOAT_WINDOW_TYPES,
        event::ClientMessage,
        property::{MapState, WmState},
        query::str_prop,
    },
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, trace, warn};

pub mod atom;
pub mod event;
mod handle;
pub mod property;
pub mod query;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub use mock::{MockXConn, StubXConn};

pub use atom::Atom;
pub use event::XEvent;
pub use property::{Prop, WindowAttributes};

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
    /// Mark this window as stacking below the given WinId
    StackBelow(WinId),
    /// Mark this window as stacking on top of its peer
    StackAbove(WinId),
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
pub trait XConn: Send {
    /// The ID of the window manager root window.
    fn root(&mut self) -> WinId;
    /// Ask the X server for the dimensions of each currently available screen.
    fn screen_details(&mut self) -> Result<Vec<Rect>>;
    /// Ask the X server for the current (x, y) coordinate of the mouse cursor.
    fn cursor_position(&mut self) -> Result<Point>;

    /// Grab the specified key and mouse states, intercepting them for processing within
    /// the window manager itself.
    fn grab(&mut self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()>;
    /// Block and wait for the next event from the X server so it can be processed.
    fn next_event(&mut self) -> Result<XEvent>;
    /// Flush any pending events to the X server.
    fn flush(&mut self);

    /// Look up the [WinId] of a given [Atom] name. If it is not currently interned, intern it.
    fn intern_atom(&mut self, atom: &str) -> Result<WinId>;
    /// Look up the string name of a given [Atom] by its [WinId].
    fn atom_name(&mut self, xid: WinId) -> Result<String>;

    /// Look up the current dimensions and position of a given client window.
    fn client_geometry(&mut self, client: WinId) -> Result<Rect>;
    /// Ask the X server for the IDs of all currently known client windows
    fn existing_clients(&mut self) -> Result<Vec<WinId>>;

    /// Map the given client window to the screen with its current geometry, making it visible.
    fn map(&mut self, client: WinId) -> Result<()>;
    /// Unmap the given client window from the screen, hiding it.
    fn unmap(&mut self, client: WinId) -> Result<()>;
    /// Kill the given client window, closing it.
    fn kill(&mut self, client: WinId) -> Result<()>;
    /// Set X input focus to be held by the given client window.
    fn focus(&mut self, client: WinId) -> Result<()>;

    /// Look up a specific property on a given client window.
    fn get_prop(&mut self, client: WinId, prop_name: &str) -> Result<Option<Prop>>;
    /// List the known property names set for a given client.
    fn list_props(&mut self, client: WinId) -> Result<Vec<String>>;
    /// Get the current [WmState] for a given client window.
    fn get_wm_state(&mut self, client: WinId) -> Result<Option<WmState>>;
    /// Request the [WindowAttributes] for a given client window from the X server.
    fn get_window_attributes(&mut self, client: WinId) -> Result<WindowAttributes>;

    /// Set the current [WmState] for a given client window.
    fn set_wm_state(&mut self, client: WinId, wm_state: WmState) -> Result<()>;
    /// Set a specific property on a given client window.
    fn set_prop(&mut self, client: WinId, name: &str, val: Prop) -> Result<()>;
    /// Delete a property for a given client window.
    fn delete_prop(&mut self, client: WinId, prop_name: &str) -> Result<()>;
    /// Set one or more [ClientAttr] for a given client window.
    fn set_client_attributes(&mut self, client: WinId, attrs: &[ClientAttr]) -> Result<()>;
    /// Set the [ClientConfig] for a given client window.
    fn set_client_config(&mut self, client: WinId, data: &[ClientConfig]) -> Result<()>;
    /// Send a [ClientMessage] to a given client.
    fn send_client_message(&mut self, msg: ClientMessage) -> Result<()>;

    /// Reposition the mouse cursor to the given (x, y) coordinates within the specified window.
    /// This method should not be called directly: use `warp_pointer_to_window` or `warp_pointer_to_screen`
    /// instead.
    fn warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()>;
}

impl ConnEvent for XEvent {
    fn requires_pointer_warp(&self) -> bool {
        !matches!(self, &XEvent::Enter(_))
    }
}

/// The check for whether or not we manage an _existing_ client is a little different from
/// whether or not we manage a _new_ client.
fn existing_client_should_be_managed<X: XConn>(x: &mut X, id: WinId) -> bool {
    let attrs = match x.get_window_attributes(id) {
        Ok(attrs) => attrs,
        _ => {
            warn!(%id, "unable to pull window attributes for client: skipping.");
            return false;
        }
    };

    let wm_state = match x.get_wm_state(id) {
        Ok(state) => state,
        _ => {
            warn!(%id, "unable to pull wm state for client: skipping.");
            return false;
        }
    };

    info!(%id, ?attrs, ?wm_state, "processing client");

    let WindowAttributes {
        override_redirect,
        map_state,
        ..
    } = attrs;

    let viewable = map_state == MapState::Viewable;
    let iconic = wm_state == Some(WmState::Iconic);

    // This condition for determining what windows we should manage is
    // taken from the `scan` function found in both dwm and XMonad.
    !override_redirect && (viewable || iconic)
}

/// Transient state needed for all XConn impls in order to track expected map/unmap events coming
/// from the Xserver.
#[derive(Debug, Default)]
pub struct XConnState {
    pub(super) mapped: HashSet<WinId>,
    pub(super) pending_unmap: HashMap<WinId, usize>,
}

impl<X> Conn for X
where
    X: XConn,
{
    type Event = XEvent;
    type State = XConnState;
    type KeyBindingKey = KeyCode;

    fn initial_state(&mut self) -> Self::State {
        XConnState::default()
    }

    #[inline]
    fn root(&mut self) -> WinId {
        self.root()
    }

    #[inline]
    fn next_event(&mut self) -> Result<XEvent> {
        self.next_event()
    }

    fn handle_event(
        &mut self,
        evt: Self::Event,
        key_bindings: &mut KeyBindings<Self>,
        mouse_bindings: &mut MouseBindings<Self>,
        state: &mut State<Self>,
    ) -> Result<()> {
        use XEvent::*;

        match evt {
            ClientMessage(m) => handle::client_message(m.clone(), state, self)?,
            ConfigureNotify(e) if e.is_root => handle::detect_screens(state, self)?,
            ConfigureNotify(_) => (), // Not currently handled
            ConfigureRequest(e) => handle::configure_request(&e, state, self)?,
            Enter(p) => handle::enter(p, state, self)?,
            Expose(_) => (), // Not currently handled
            FocusIn(id) => handle::focus_in(id, state, self)?,
            Destroy(id) => handle::destroy(id, state, self)?,
            KeyPress(code) => handle::keypress(code, key_bindings, state, self)?,
            Leave(p) => handle::leave(p, state, self)?,
            MappingNotify => handle::mapping_notify(key_bindings, mouse_bindings, self)?,
            MapRequest(xid) => handle::map_request(xid, state, self)?,
            MouseEvent(e) => handle::mouse_event(e.clone(), mouse_bindings, state, self)?,
            MotionNotify(e) => handle::motion_event(e.clone(), mouse_bindings, state, self)?,
            PropertyNotify(_) => (), // Not currently handled
            RandrNotify => handle::detect_screens(state, self)?,
            ScreenChange => handle::screen_change(state, self)?,
            UnmapNotify(id) => handle::unmap_notify(id, state, self)?,

            _ => (), // XEvent is non-exhaustive
        }

        Ok(())
    }

    #[inline]
    fn flush(&mut self) {
        self.flush();
    }

    #[inline]
    fn grab(&mut self, key_codes: &[KeyCode], mouse_states: &[MouseState]) -> Result<()> {
        self.grab(key_codes, mouse_states)
    }

    #[inline]
    fn screen_details(&mut self) -> Result<Vec<Rect>> {
        self.screen_details()
    }

    #[inline]
    fn cursor_position(&mut self) -> Result<Point> {
        self.cursor_position()
    }

    #[inline]
    fn warp_pointer(&mut self, id: WinId, x: i16, y: i16) -> Result<()> {
        self.warp_pointer(id, x, y)
    }

    #[inline]
    fn existing_clients(&mut self) -> Result<Vec<WinId>> {
        self.existing_clients()
    }

    fn position_client(&mut self, id: WinId, mut r: Rect) -> Result<()> {
        let p = Atom::WmNormalHints.as_ref();
        if let Ok(Some(Prop::WmNormalHints(hints))) = self.get_prop(id, p) {
            trace!(%id, ?hints, "client has WmNormalHints: applying size hints");
            r = hints.apply_to(r);
        }

        trace!(%id, ?r, "positioning client");
        self.set_client_config(id, &[ClientConfig::Position(r)])
    }

    fn show_client(&mut self, id: WinId, state: &mut State<Self>) -> Result<()> {
        self.set_wm_state(id, WmState::Normal)?;
        self.map(id)?;

        if state.client_set.contains(&id) {
            state.conn_state.mapped.insert(id);
        }

        Ok(())
    }

    fn hide_client(&mut self, id: WinId, state: &mut State<Self>) -> Result<()> {
        if !state.conn_state.mapped.contains(&id) {
            return Ok(());
        }

        self.set_client_attributes(id, &[ClientAttr::ClientUnmapMask])?;
        self.unmap(id)?;
        self.set_client_attributes(id, &[ClientAttr::ClientEventMask])?;
        self.set_wm_state(id, WmState::Iconic)?;

        state.conn_state.mapped.remove(&id);
        state
            .conn_state
            .pending_unmap
            .entry(id)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        Ok(())
    }

    #[inline]
    fn withdraw_client(&mut self, id: WinId) -> Result<()> {
        self.set_wm_state(id, WmState::Withdrawn)
    }

    #[inline]
    fn kill_client(&mut self, id: WinId) -> Result<()> {
        self.kill(id)
    }

    #[inline]
    fn focus_client(&mut self, id: WinId) -> Result<()> {
        self.focus(id)
    }

    #[inline]
    fn client_geometry(&mut self, id: WinId) -> Result<Rect> {
        self.client_geometry(id)
    }

    fn client_title(&mut self, id: WinId) -> Result<String> {
        match str_prop(Atom::WmName, id, self) {
            Ok(Some(mut strs)) => Ok(strs.remove(0)),
            _ => match str_prop(Atom::NetWmName, id, self)? {
                Some(mut strs) => Ok(strs.remove(0)),
                None => Ok("".to_owned()),
            },
        }
    }

    fn client_pid(&mut self, id: WinId) -> Option<u32> {
        if let Ok(Some(Prop::Cardinal(vals))) = self.get_prop(id, "_NET_WM_PID") {
            Some(vals[0])
        } else {
            None
        }
    }

    fn client_should_float(&mut self, id: WinId, floating_classes: &[String]) -> bool {
        trace!(%id, "fetching WmClass prop");
        if let Ok(Some(Prop::UTF8String(strs))) = self.get_prop(id, Atom::WmClass.as_ref())
            && strs.iter().any(|c| floating_classes.contains(c))
        {
            debug!(%id, ?floating_classes, "window has a floating class: setting to floating state");
            return true;
        }

        trace!(%id, "fetching NetWmWindowType prop");
        let window_types = match self.get_prop(id, Atom::NetWmWindowType.as_ref()) {
            Ok(tys) => tys,
            _ => return false,
        };
        debug!(?window_types, "id window types");

        let float_types: Vec<&str> = AUTO_FLOAT_WINDOW_TYPES.iter().map(|a| a.as_ref()).collect();

        match window_types {
            Some(Prop::Atom(atoms)) => atoms.iter().any(|a| float_types.contains(&a.as_ref())),
            _ => false,
        }
    }

    fn client_should_be_managed(&mut self, id: WinId) -> bool {
        match self.get_window_attributes(id) {
            Ok(attrs) => !attrs.override_redirect,
            _ => {
                warn!(%id, "unable to pull window attributes for client: skipping.");
                false
            }
        }
    }

    fn client_is_fullscreen(&mut self, id: WinId) -> bool {
        let net_wm_state = Atom::NetWmState.as_ref();
        let full_screen = match self.intern_atom(Atom::NetWmStateFullscreen.as_ref()) {
            Ok(atom) => atom,
            _ => return false,
        };

        let wstate = match self.get_prop(id, net_wm_state) {
            Ok(Some(Prop::Cardinal(vals))) => vals,
            _ => vec![],
        };

        wstate.contains(&full_screen)
    }

    fn client_transient_parent(&mut self, id: WinId) -> Option<WinId> {
        match self.get_prop(id, Atom::WmTransientFor.as_ref()).ok()?? {
            Prop::Window(ids) => Some(ids[0]),
            _ => None,
        }
    }

    fn set_client_border_color(&mut self, id: WinId, color: impl Into<Color>) -> Result<()> {
        let color = color.into();
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color.argb_u32())])
    }

    fn set_initial_properties(&mut self, id: WinId, config: &Config<Self>) -> Result<()> {
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

        self.set_wm_state(id, WmState::Iconic)?;
        self.set_client_attributes(id, attrs)?;
        self.set_client_config(id, conf)
    }

    fn restack<'a, I>(&mut self, mut ids: I) -> Result<()>
    where
        WinId: 'a,
        I: Iterator<Item = &'a WinId>,
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

    // A "best effort" attempt to manage existing clients on the workspaces they were present
    // on previously. This is not guaranteed to preserve the stack order or correctly handle
    // any clients that were on invisible workspaces / workspaces that no longer exist.
    //
    // NOTE: the check for if each client is already in state is in case a startup hook has
    //       pre-managed clients for us. In that case we want to avoid stomping on
    //       anything that they have set up.
    #[tracing::instrument(level = "info", skip(state, self))]
    fn manage_existing_clients(&mut self, state: &mut State<Self>) -> Result<()> {
        // We're not guaranteed that workspace indices are _always_ continuous from 0..n
        // so we explicitly map tags to indices instead.
        // We also exclude hidden workspaces as those can contain windows which are
        // externally managed by a user written extension, which can lead to malformed
        // internal state for those extensions when they restart.
        let ws_map: HashMap<usize, String> = state
            .client_set
            .non_hidden_workspaces()
            .map(|w| (w.id, w.tag.clone()))
            .collect();

        let first_tag = state.client_set.ordered_tags()[0].clone();

        for id in self.existing_clients()? {
            if !state.client_set.contains(&id) && existing_client_should_be_managed(self, id) {
                let workspace_id = match self.get_prop(id, Atom::NetWmDesktop.as_ref()) {
                    Ok(Some(Prop::Cardinal(ids))) => ids[0] as usize,
                    _ => 0, // we know that we always have at least one workspace
                };

                let tag = ws_map.get(&workspace_id).unwrap_or(&first_tag);
                let title = self.client_title(id)?;
                info!(%id, %title, %tag, "attempting to manage existing client");
                manage_without_refresh(id, Some(tag), state, self)?;
            }
        }

        // If EWMH is enabled then we should have this property set to tell us what the previously
        // active client was. If that client is not in the client set or the property is not set we
        // default to forcing focus to the first available tag and whatever active client we have there
        // as that is where we will have placed all existing clients.
        match self.get_prop(state.root, Atom::NetActiveWindow.as_ref()) {
            Ok(Some(Prop::Window(ids))) if state.client_set.contains(&ids[0]) => {
                let id = ids[0];
                info!(%id, "focusing _NET_ACTIVE_WINDOW client");
                state.client_set.focus_client(&id);
            }
            _ => {
                info!(%first_tag, "unable to determine an active window: focusing first tag");
                state.client_set.focus_tag(&first_tag);
            }
        };

        info!("triggering refresh");
        self.refresh(state)
    }
}
