use crate::{
    bindings::{KeyCode, MouseState},
    core::{ClientSet, Config, State},
    geometry::{Point, Rect},
    layout::messages::control::Hide,
    pure::Diff,
    x::{
        atom::{Atom, AUTO_FLOAT_WINDOW_TYPES},
        event::ClientMessage,
        property::{Prop, WmState},
    },
    Color, Result, Xid,
};
use std::collections::{HashMap, HashSet};
use tracing::trace;

pub mod atom;
pub mod event;
pub mod property;

pub use event::XEvent;
pub use property::WindowAttributes;

pub type ScreenId = usize;

pub fn id<T>(_: &mut T) {}

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
    /// Mark this window as stacking on top of its peers
    StackAbove,
}

/// Attributes for an X11 client window (not all are curently implemented)
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClientAttr {
    /// Border color as an argb hex value
    BorderColor(u32),
    /// Set the pre-defined client event mask
    ClientEventMask,
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

    fn float_location(&self, client: Xid) -> Result<Rect>;

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

    fn warp_cursor(&self, p: Point) -> Result<()>;
}

// Derivable methods for XConn that should never be given a different implementation
pub trait XConnExt: XConn + Sized {
    /// Kill the focused client if there is one
    fn kill_focused<E>(&self, state: &mut State<Self, E>) -> Result<()>
    where
        E: Send + Sync + 'static,
    {
        if let Some(&id) = state.client_set.current_client() {
            self.kill(id)?;
        }

        Ok(())
    }

    fn manage<E>(&self, client: Xid, state: &mut State<Self, E>) -> Result<()>
    where
        E: Send + Sync + 'static,
    {
        trace!(%client, "managing new client");
        let should_float = self.client_should_float(client, &state.config.floating_classes)?;
        let r = self.float_location(client)?;
        let mut hook = state.config.manage_hook.take();

        trace!("calling modify and refresh");
        let res = self.modify_and_refresh(state, |cs| {
            cs.insert(client);
            if should_float {
                cs.float_unchecked(client, r);
            }

            // TODO: should this be called here? Or in a second refresh?
            if let Some(ref mut h) = hook {
                trace!("running user manage hook");
                h.call(client, cs, self);
            }
        });

        // make sure we reset the manage hook before returning any errors
        state.config.manage_hook = hook;

        res
    }

    fn unmanage<E>(&self, client: Xid, state: &mut State<Self, E>) -> Result<()>
    where
        E: Send + Sync + 'static,
    {
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

        // TODO: double check this swap-out around structureNotifyMask
        // io $ do selectInput d w (cMask .&. complement structureNotifyMask)
        //         unmapWindow d w
        //         selectInput d w cMask

        self.unmap(client)?;
        self.set_wm_state(client, WmState::Normal)?;

        mapped.remove(&client);
        pending_unmap
            .entry(client)
            .and_modify(|count| *count += 1)
            .or_insert(1);

        Ok(())
    }

    fn refresh<E>(&self, state: &mut State<Self, E>) -> Result<()>
    where
        E: Send + Sync + 'static,
    {
        self.modify_and_refresh(state, id)
    }

    /// Apply a pure function that modifies a [ClientSet] and then handle refreshing the
    /// Window Manager state and associated X11 calls.
    fn modify_and_refresh<F, E>(&self, state: &mut State<Self, E>, mut f: F) -> Result<()>
    where
        F: FnMut(&mut ClientSet),
        E: Send + Sync + 'static,
    {
        trace!("taking state snapshot");
        let ss = state.client_set.snapshot();

        f(&mut state.client_set); // NOTE: mutating the existing state

        let positions = state.client_set.visible_client_positions();
        let diff = Diff::from_raw(ss, &state.client_set, &positions);

        for c in diff.new.into_iter() {
            self.set_initial_properties(c, &state.config)?;
        }

        if let Some(focused) = diff.old_focus {
            self.set_client_border_color(focused, state.config.normal_border)?;
        }

        state
            .client_set
            .iter_hidden_workspaces_mut()
            .filter(|w| diff.previous_visible_tags.contains(&w.tag))
            .for_each(|ws| ws.broadcast_message(Hide));

        for (c, r) in positions {
            trace!(?c, ?r, "positioning client");
            self.position_client(c, r)?;
        }

        if let Some(&focused) = state.client_set.current_client() {
            trace!(?focused, "setting border for focused client");
            self.set_client_border_color(focused, state.config.focused_border)?;
        }

        for c in diff.visible.into_iter() {
            trace!(?c, "revealing client");
            self.reveal(c, &state.client_set, &mut state.mapped)?;
        }

        self.focus(
            state
                .client_set
                .current_client()
                .copied()
                .unwrap_or(state.root),
        )?;

        for c in diff.hidden.into_iter() {
            trace!(?c, "hiding client");
            self.hide(c, &mut state.mapped, &mut state.pending_unmap)?;
        }

        for c in diff.withdrawn.into_iter() {
            trace!(?c, "setting withdrawn state for client");
            self.set_wm_state(c, WmState::Withdrawn)?;
        }

        // TODO:
        // clear enterWindow events from the event queue if this was because of mouse focus (?)

        let mut hook = state.config.refresh_hook.take();
        if let Some(ref mut h) = hook {
            trace!("running user refresh hook");
            h.call(state, self);
        }
        state.config.refresh_hook = hook;

        Ok(())
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
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color.rgba_u32())])
    }

    fn set_initial_properties<E>(&self, client: Xid, config: &Config<Self, E>) -> Result<()>
    where
        E: Send + Sync + 'static,
    {
        let Config {
            normal_border,
            border_width,
            ..
        } = config;

        let conf = &[ClientConfig::BorderPx(*border_width)];
        let attrs = &[
            ClientAttr::ClientEventMask,
            ClientAttr::BorderColor(normal_border.rgba_u32()),
        ];

        self.set_wm_state(client, WmState::Iconic)?;
        self.set_client_attributes(client, attrs)?;
        self.set_client_config(client, conf)
    }

    fn position_client(&self, client: Xid, r: Rect) -> Result<()> {
        self.set_client_config(
            client,
            &[ClientConfig::Position(r), ClientConfig::StackAbove],
        )
    }

    fn set_active_client<E>(&self, client: Xid, state: &mut State<Self, E>) -> Result<()>
    where
        E: Send + Sync + 'static,
    {
        self.modify_and_refresh(state, |cs| cs.focus_client(&client))
    }
}

// Auto impl XConnExt for all XConn impls
impl<T> XConnExt for T where T: XConn {}
