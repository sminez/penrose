use crate::{
    core::{ClientDiff, ClientSet, Config, State},
    geometry::Rect,
    layout::messages::control::Hide,
    x::{
        atom::{Atom, AUTO_FLOAT_WINDOW_TYPES},
        property::{Prop, WmState},
    },
    Color, Xid,
};
use tracing::trace;

pub mod atom;
pub mod event;
pub mod property;

pub use event::XEvent;

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
    fn get_screen_details(&self) -> Vec<Rect>;
    fn get_prop(&self, client: Xid, prop_name: &str) -> Option<Prop>;

    fn float_location(&self, client: Xid) -> (ScreenId, Rect);

    fn hide(&self, client: Xid);
    fn reveal(&self, client: Xid);
    fn kill(&self, client: Xid);
    fn focus(&self, client: Xid);

    fn set_wm_state(&self, client: Xid, wm_state: WmState);
    fn set_client_attributes(&self, id: Xid, data: &[ClientAttr]);
    fn set_client_config(&self, client: Xid, data: &[ClientConfig]);

    fn tile_client(&self, client: Xid, r: Rect);
}

// Derivable methods for XConn that should never be given a different implementation
pub trait XConnExt: XConn {
    /// Kill the focused client if there is one
    fn kill_focused(&self, state: &mut State) {
        if let Some(&id) = state.client_set.current_client() {
            self.kill(id)
        }
    }

    fn manage(&self, client: Xid, state: &mut State) {
        let should_float = self.client_should_float(client, &state.config.floating_classes);
        let (_, r) = self.float_location(client);

        self.modify_and_refresh(state, |cs| {
            cs.insert(client);
            if should_float {
                cs.float_unchecked(client, r);
            }
            // TODO: run manage hook
        })
    }

    fn unmanage(&self, client: Xid, state: &mut State) {
        self.modify_and_refresh(state, |cs| {
            cs.remove_client(&client);
        })
    }

    fn refresh(&self, state: &mut State) {
        self.modify_and_refresh(state, id)
    }

    /// Apply a pure function that modifies a [ClientSet] and then handle refreshing the
    /// Window Manager state and associated X11 calls.
    fn modify_and_refresh<F>(&self, state: &mut State, mut f: F)
    where
        F: FnMut(&mut ClientSet) -> (),
    {
        let State {
            config,
            root,
            client_set,
            ..
        } = state;

        let ss = client_set.snapshot();

        f(client_set); // NOTE: mutating the existing state

        let positions = client_set.visible_client_positions();
        let diff = ClientDiff::from_raw(ss, *root, &client_set, &positions);

        diff.new
            .into_iter()
            .for_each(|c| self.set_initial_properties(c, &config));

        if let Some(focused) = diff.old_focus {
            self.set_client_border_color(focused, config.normal_border);
        }

        client_set
            .iter_hidden_workspaces_mut()
            .filter(|w| diff.previous_visible_tags.contains(&w.tag))
            .for_each(|ws| ws.broadcast_message(Hide));

        self.position_clients(&positions);

        if diff.new_focus != *root {
            self.set_client_border_color(diff.new_focus, config.focused_border);
        }

        diff.visible.into_iter().for_each(|c| self.reveal(c));

        self.focus(diff.new_focus);

        diff.hidden.into_iter().for_each(|c| self.hide(c));

        diff.withdrawn
            .into_iter()
            .for_each(|c| self.set_wm_state(c, WmState::Withdrawn));

        // TODO:
        // clear enterWindow events from the event queue if this was because of mouse focus (?)
        // run the user's event hook (XMonad calls this 'logHook'. Need a better name)
    }

    fn client_should_float(&self, client: Xid, floating_classes: &[String]) -> bool {
        if let Some(prop) = self.get_prop(client, Atom::WmTransientFor.as_ref()) {
            trace!(?prop, "window is transient: setting to floating state");
            return true;
        }

        if let Some(Prop::UTF8String(strs)) = self.get_prop(client, Atom::WmClass.as_ref()) {
            if strs.iter().any(|c| floating_classes.contains(c)) {
                return true;
            }
        }

        let float_types: Vec<&str> = AUTO_FLOAT_WINDOW_TYPES.iter().map(|a| a.as_ref()).collect();
        if let Some(Prop::Atom(atoms)) = self.get_prop(client, Atom::NetWmWindowType.as_ref()) {
            atoms.iter().any(|a| float_types.contains(&a.as_ref()))
        } else {
            false
        }
    }

    fn set_client_border_color<C>(&self, id: Xid, color: C)
    where
        C: Into<Color>,
    {
        let color = color.into();
        self.set_client_attributes(id, &[ClientAttr::BorderColor(color.rgba_u32())]);
    }

    fn set_initial_properties(&self, client: Xid, config: &Config) {
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

        self.set_wm_state(client, WmState::Iconic);
        self.set_client_attributes(client, attrs);
        self.set_client_config(client, conf);
    }

    fn position_clients(&self, positions: &[(Xid, Rect)]) {
        for &(c, r) in positions {
            self.set_client_config(c, &[ClientConfig::Position(r), ClientConfig::StackAbove]);
        }
    }
}

// Auto impl XConnExt for all XConn impls
impl<T: ?Sized> XConnExt for T where T: XConn {}
