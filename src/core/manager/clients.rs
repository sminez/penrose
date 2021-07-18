//! State and management of clients being managed by Penrose.
use crate::{
    core::{
        client::Client,
        data_types::Region,
        hooks::HookName,
        layout::LayoutConf,
        manager::{event::EventAction, util::pad_region},
        ring::Selector,
        workspace::ArrangeActions,
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(super) struct Clients {
    inner: HashMap<Xid, Client>,
    focused_client_id: Option<Xid>,
    focused_border: Color,
    unfocused_border: Color,
}

impl Clients {
    pub fn new(focused_border: impl Into<Color>, unfocused_border: impl Into<Color>) -> Self {
        Self {
            inner: HashMap::new(),
            focused_client_id: None,
            focused_border: focused_border.into(),
            unfocused_border: unfocused_border.into(),
        }
    }

    pub fn is_known(&self, id: Xid) -> bool {
        self.inner.contains_key(&id)
    }

    pub fn focused_client_id(&self) -> Option<Xid> {
        self.focused_client_id
    }

    pub fn focused_client(&self) -> Option<&Client> {
        self.focused_client_id.and_then(|id| self.inner.get(&id))
    }

    pub fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focused_client_id
            .and_then(move |id| self.inner.get_mut(&id))
    }

    pub fn client(&self, selector: &Selector<'_, Client>) -> Option<&Client> {
        match selector {
            Selector::Focused | Selector::Any => self.focused_client(),
            Selector::WinId(id) => self.inner.get(&id),
            Selector::Condition(f) => self.inner.iter().find(|(_, v)| f(v)).map(|(_, v)| v),
            Selector::Index(i) => self.inner.iter().nth(*i).map(|(_, c)| c),
        }
    }

    pub fn client_mut(&mut self, selector: &Selector<'_, Client>) -> Option<&mut Client> {
        match selector {
            Selector::Focused | Selector::Any => self.focused_client_mut(),
            Selector::WinId(id) => self.inner.get_mut(&id),
            Selector::Condition(f) => self.inner.iter_mut().find(|(_, v)| f(v)).map(|(_, v)| v),
            Selector::Index(i) => self.inner.iter_mut().nth(*i).map(|(_, c)| c),
        }
    }

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

    pub fn matching_clients_mut(&mut self, selector: &Selector<'_, Client>) -> Vec<&mut Client> {
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

    pub fn set_focused<X>(&mut self, id: Xid, conn: &X) -> Option<Xid>
    where
        X: XClientConfig,
    {
        let prev = self.focused_client_id;
        self.focused_client_id = Some(id);

        if let Some(prev_id) = prev {
            if id != prev_id {
                self.client_lost_focus(prev_id, conn);
            }
        }

        prev
    }

    #[allow(dead_code)]
    pub fn clear_focused(&mut self) {
        self.focused_client_id = None
    }

    pub fn insert(&mut self, id: Xid, c: Client) -> Option<Client> {
        self.inner.insert(id, c)
    }

    pub fn remove(&mut self, id: Xid) -> Option<Client> {
        if self.focused_client_id == Some(id) {
            self.focused_client_id = None;
        }

        self.inner.remove(&id)
    }

    pub fn get(&self, id: Xid) -> Option<&Client> {
        self.inner.get(&id)
    }

    pub fn get_mut(&mut self, id: Xid) -> Option<&mut Client> {
        self.inner.get_mut(&id)
    }

    pub fn set_client_workspace(&mut self, id: Xid, wix: usize) {
        self.inner.entry(id).and_modify(|c| c.set_workspace(wix));
    }

    pub fn map_if_needed<X>(&mut self, id: Xid, conn: &X) -> Result<()>
    where
        X: XClientHandler,
    {
        Ok(conn.map_client_if_needed(self.inner.get_mut(&id))?)
    }

    pub fn unmap_if_needed<X>(&mut self, id: Xid, conn: &X) -> Result<()>
    where
        X: XClientHandler,
    {
        Ok(conn.unmap_client_if_needed(self.inner.get_mut(&id))?)
    }

    // The index of the [Workspace] holding the requested X window ID. This can return None if
    // the id does not map to a [WindowManager] managed [Client] which happens if the window
    // is unmanaged (e.g. a dock or toolbar) or if a client [Hook] has requested ownership
    // of that particular [Client].
    pub fn workspace_index_for_client(&self, id: Xid) -> Option<usize> {
        self.inner.get(&id).map(|c| c.workspace())
    }

    pub fn clients_for_workspace(&self, wix: usize) -> Vec<&Client> {
        self.matching_clients(&Selector::Condition(&|c: &Client| c.workspace == wix))
    }

    pub fn all_known_ids(&self) -> Vec<Xid> {
        self.inner.keys().copied().collect()
    }

    pub fn modify(&mut self, id: Xid, f: impl Fn(&mut Client)) {
        self.inner.entry(id).and_modify(f);
    }

    // Set X focus to the requested client if it accepts focus, otherwise send a
    // 'take focus' event for the client to process
    pub fn set_x_focus<X>(&self, id: Xid, accepts_focus: bool, conn: &X) -> Result<()>
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

    pub fn focus_in<X>(&self, id: Xid, conn: &X) -> Result<()>
    where
        X: XState + XEventHandler + XClientConfig + XClientHandler + XClientProperties,
    {
        let accepts_focus = match self.inner.get(&id) {
            Some(client) => client.accepts_focus,
            None => conn.client_accepts_focus(id),
        };

        self.set_x_focus(id, accepts_focus, conn)
    }

    // The given X window ID lost focus according to the X server
    #[tracing::instrument(level = "trace", skip(self, conn))]
    pub fn client_lost_focus<X>(&mut self, id: Xid, conn: &X)
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
    pub fn client_name_changed<X>(
        &mut self,
        id: Xid,
        is_root: bool,
        conn: &X,
    ) -> Result<EventAction>
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

    pub fn apply_arrange_actions<X>(
        &mut self,
        actions: ArrangeActions,
        lc: &LayoutConf,
        border_px: u32,
        gap_px: u32,
        conn: &X,
    ) -> Result<()>
    where
        X: XClientHandler + XClientConfig,
    {
        // Tile first then place floating clients on top
        for (id, region) in actions.actions {
            trace!(id, ?region, "positioning client");
            if let Some(region) = region {
                let reg = pad_region(&region, lc.gapless, gap_px, border_px);
                conn.position_client(id, reg, border_px, false)?;
                self.map_if_needed(id, conn)?;
            } else {
                self.unmap_if_needed(id, conn)?;
            }
        }

        for id in actions.floating {
            debug!(id, "mapping floating client above tiled");
            conn.raise_client(id)?;
        }

        Ok(())
    }

    pub fn toggle_fullscreen<X>(
        &mut self,
        id: Xid,
        wix: usize,
        workspace_clients: &[Xid],
        screen_size: Region,
        conn: &X,
    ) -> Result<Vec<EventAction>>
    where
        X: XClientHandler + XClientProperties + XClientConfig,
    {
        let client_currently_fullscreen = match self.get(id) {
            Some(c) => c.fullscreen,
            None => {
                warn!(id, "attempt to make unknown client fullscreen");
                return Ok(vec![]);
            }
        };

        conn.toggle_client_fullscreen(id, client_currently_fullscreen)?;

        for &i in workspace_clients.iter() {
            if client_currently_fullscreen {
                if i == id {
                    self.inner.entry(id).and_modify(|c| c.fullscreen = false);
                } else {
                    self.map_if_needed(i, conn)?;
                }
            // client was not fullscreen
            } else if i == id {
                conn.position_client(id, screen_size, 0, false)?;
                let is_known = self.is_known(id);
                if is_known {
                    self.map_if_needed(id, conn)?;
                    self.modify(id, |c| c.fullscreen = true);
                }
            } else {
                self.unmap_if_needed(i, conn)?;
            }
        }

        Ok(if client_currently_fullscreen {
            vec![EventAction::LayoutWorkspace(wix)]
        } else {
            vec![]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::xconnection::{self, *};
    use std::cell::Cell;

    #[test]
    fn client_lost_focus_on_focused_clears_focused_client_id() {
        let conn = MockXConn::new(vec![], vec![], vec![]);
        let mut clients = Clients::new(0xffffff, 0x000000);

        clients.focused_client_id = Some(42);
        clients.client_lost_focus(42, &conn);
        assert!(clients.focused_client_id.is_none());
    }

    struct RecordingXConn {
        positions: Cell<Vec<(Xid, Region)>>,
        maps: Cell<Vec<Xid>>,
        unmaps: Cell<Vec<Xid>>,
    }

    impl RecordingXConn {
        fn init() -> Self {
            Self {
                positions: Cell::new(Vec::new()),
                maps: Cell::new(Vec::new()),
                unmaps: Cell::new(Vec::new()),
            }
        }
    }

    impl StubXClientProperties for RecordingXConn {}

    impl StubXClientHandler for RecordingXConn {
        fn mock_map_client(&self, id: Xid) -> xconnection::Result<()> {
            let mut v = self.maps.take();
            v.push(id);
            self.maps.set(v);
            Ok(())
        }

        fn mock_unmap_client(&self, id: Xid) -> xconnection::Result<()> {
            let mut v = self.unmaps.take();
            v.push(id);
            self.unmaps.set(v);
            Ok(())
        }
    }

    impl StubXClientConfig for RecordingXConn {
        fn mock_position_client(
            &self,
            id: Xid,
            r: Region,
            _: u32,
            _: bool,
        ) -> xconnection::Result<()> {
            let mut v = self.positions.take();
            v.push((id, r));
            self.positions.set(v);
            Ok(())
        }
    }

    test_cases! {
        toggle_fullscreen;
        args: (
            n_clients: u32,
            fullscreen: Option<Xid>,
            target: Xid,
            unmapped: &[Xid],
            should_apply_layout: bool,
            expected_positions: Vec<Xid>,
            expected_maps: Vec<Xid>,
            expected_unmaps: Vec<Xid>,
        );

        case: single_client_on => (1, None, 0, &[], false, vec![0], vec![], vec![]);
        case: single_client_off => (1, Some(0), 0, &[], true, vec![], vec![], vec![]);
        case: multiple_clients_on => (4, None, 1, &[], false, vec![1], vec![], vec![0, 2, 3]);
        case: multiple_clients_off => (4, Some(1), 1, &[0, 2, 3], true, vec![], vec![0, 2, 3], vec![]);

        body: {
            let conn = RecordingXConn::init();
            let ids: Vec<Xid> = (0..n_clients).collect();

            let mut clients = Clients {
                inner: ids.iter()
                .map(|&id| {
                    let mut client = Client::new(&conn, id, 0, &[]);
                    client.mapped = true;
                    (id, client)
                })
                .collect(),
                focused_client_id: None,
                focused_border: 0xffffff.into(),
                unfocused_border: 0x000000.into(),
            };

            let r = Region::new(0, 0, 1000, 800);
            let expected_positions: Vec<_> = expected_positions.iter().map(|id| (*id, r)).collect();

            for id in unmapped {
                clients.modify(*id, |c| c.mapped = false);
            }

            if let Some(id) = fullscreen {
                clients.modify(id, |c| c.fullscreen = true);
            }

            let events = clients.toggle_fullscreen(target, 42, &ids, r, &conn).unwrap();

            assert_eq!(!events.is_empty(), should_apply_layout);
            assert_eq!(conn.positions.take(), expected_positions);
            assert_eq!(conn.maps.take(), expected_maps);
            assert_eq!(conn.unmaps.take(), expected_unmaps);
        }
    }
}
