//! Management of clients
use crate::{
    core::{
        client::Client,
        hooks::HookName,
        manager::event::EventAction,
        ring::Selector,
        xconnection::{
            Atom, ClientMessageKind, Prop, XClientConfig, XClientHandler, XClientProperties,
            XEventHandler, XState, Xid,
        },
    },
    draw::Color,
    Result,
};
use std::collections::HashMap;
use tracing::{trace, warn};

/// State and management of clients being managed by Penrose.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Clients {
    inner: HashMap<Xid, Client>,
    pub(super) focused_client_id: Option<Xid>,
    focused_border: Color,
    unfocused_border: Color,
}

impl Clients {
    /// Create a new empty client map
    pub fn new(focused_border: Color, unfocused_border: Color) -> Self {
        Self {
            inner: HashMap::new(),
            focused_client_id: None,
            focused_border,
            unfocused_border,
        }
    }

    /// A reference to the current focused [Client] if there is one
    pub fn focused_client(&self) -> Option<&Client> {
        self.focused_client_id
            .and_then(move |id| self.inner.get(&id))
    }

    /// A mutable reference to the current focused [Client] if there is one
    pub(super) fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focused_client_id
            .and_then(move |id| self.inner.get_mut(&id))
    }

    /// A reference to the first [Client] matching the given [Selector]
    pub fn client(&self, selector: &Selector<'_, Client>) -> Option<&Client> {
        match selector {
            Selector::Focused | Selector::Any => self.focused_client(),
            Selector::WinId(id) => self.inner.get(&id),
            Selector::Condition(f) => self.inner.iter().find(|(_, v)| f(v)).map(|(_, v)| v),
            Selector::Index(i) => self.inner.iter().nth(*i).map(|(_, c)| c),
        }
    }

    /// A mutable reference to the first [Client] matching the given [Selector]
    pub fn client_mut(&mut self, selector: &Selector<'_, Client>) -> Option<&mut Client> {
        match selector {
            Selector::Focused | Selector::Any => self.focused_client_mut(),
            Selector::WinId(id) => self.inner.get_mut(&id),
            Selector::Condition(f) => self.inner.iter_mut().find(|(_, v)| f(v)).map(|(_, v)| v),
            Selector::Index(i) => self.inner.iter_mut().nth(*i).map(|(_, c)| c),
        }
    }

    /// References to every [Client] matching the given [Selector]
    pub fn matching_clients(&self, selector: &Selector<'_, Client>) -> Vec<&Client> {
        let mut clients: Vec<&Client> = match selector {
            Selector::Any => self.inner.values().collect(),
            Selector::Focused => self.focused_client().into_iter().collect(),
            Selector::WinId(id) => self.inner.get(&id).into_iter().collect(),
            Selector::Condition(f) => self.inner.values().filter(|v| f(v)).collect(),
            _ => self.client(selector).into_iter().collect(),
        };

        clients.sort_unstable_by_key(|c| c.id());
        clients
    }

    /// Mutable references to every [Client] matching the given [Selector]
    pub fn all_clients_mut(&mut self, selector: &Selector<'_, Client>) -> Vec<&mut Client> {
        let mut clients: Vec<&mut Client> = match selector {
            Selector::Any => self.inner.values_mut().collect(),
            Selector::Focused => self.focused_client_mut().into_iter().collect(),
            Selector::WinId(id) => self.inner.get_mut(&id).into_iter().collect(),
            Selector::Condition(f) => self.inner.values_mut().filter(|v| f(v)).collect(),
            _ => self.client_mut(selector).into_iter().collect(),
        };

        clients.sort_unstable_by_key(|c| c.id());
        clients
    }

    // The index of the [Workspace] holding the requested X window ID. This can return None if
    // the id does not map to a [WindowManager] managed [Client] which happens if the window
    // is unmanaged (e.g. a dock or toolbar) or if a client [Hook] has requested ownership
    // of that particular [Client].
    pub(super) fn workspace_index_for_client(&self, id: Xid) -> Option<usize> {
        self.inner.get(&id).map(|c| c.workspace())
    }

    // Set X focus to the requested client if it accepts focus, otherwise send a
    // 'take focus' event for the client to process
    pub(super) fn set_focus<X>(&self, id: Xid, accepts_focus: bool, conn: &X) -> Result<()>
    where
        X: XState + XEventHandler + XClientConfig + XClientHandler + XClientProperties,
    {
        trace!(id, accepts_focus, "setting focus");
        if accepts_focus {
            if let Err(e) = conn.focus_client(id) {
                warn!("unable to focus client {}: {}", id, e);
            }
            conn.change_prop(
                conn.root(),
                Atom::NetActiveWindow.as_ref(),
                Prop::Window(vec![id]),
            )?;
            let fb = self.focused_border;
            if let Err(e) = conn.set_client_border_color(id, fb) {
                warn!("unable to set client border color for {}: {}", id, e);
            }
        } else {
            let msg = ClientMessageKind::TakeFocus(id).as_message(conn)?;
            conn.send_client_event(msg)?;
        }

        // TODO: should this be running the FocusChange hook?
        Ok(())
    }

    pub(super) fn focus_in<X>(&self, id: Xid, conn: &X) -> Result<()>
    where
        X: XState + XEventHandler + XClientConfig + XClientHandler + XClientProperties,
    {
        let accepts_focus = match self.inner.get(&id) {
            Some(client) => client.accepts_focus,
            None => conn.client_accepts_focus(id),
        };

        self.set_focus(id, accepts_focus, conn)
    }

    // The given X window ID lost focus according to the X server
    #[tracing::instrument(level = "trace", skip(self, conn))]
    pub(super) fn client_lost_focus<X>(&mut self, id: Xid, conn: &X)
    where
        X: XClientConfig,
    {
        if self.focused_client_id == Some(id) {
            self.focused_client_id = None;
        }

        if self.inner.contains_key(&id) {
            let ub = self.unfocused_border;
            // The target window may have lost focus because it has just been closed and
            // we have not yet updated our state.
            conn.set_client_border_color(id, ub).unwrap_or(());
        }
    }

    // The given window ID has had its EWMH name updated by something
    pub(super) fn client_name_changed<X>(
        &mut self,
        id: Xid,
        is_root: bool,
        conn: &X,
    ) -> Result<EventAction<'_>>
    where
        X: XClientProperties,
    {
        let name = conn.client_name(id)?;
        if !is_root {
            if let Some(c) = self.inner.get_mut(&id) {
                c.set_name(&name)
            }
        }

        Ok(EventAction::RunHook(HookName::ClientNameUpdated(
            id, name, is_root,
        )))
    }
}
