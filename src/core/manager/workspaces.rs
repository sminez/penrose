//! Management of workspaces
use crate::{
    core::{
        client::Client,
        data_types::{Change, Region},
        hooks::HookName,
        layout::LayoutConf,
        manager::EventAction,
        ring::{Direction, InsertPoint, Ring, Selector},
        workspace::{ArrangeActions, Workspace},
        xconnection::Xid,
    },
    Result,
};

use std::ops::{Deref, DerefMut};

#[cfg(feature = "serde")]
use std::collections::HashMap;

#[cfg(feature = "serde")]
use crate::core::layout::LayoutFunc;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(super) struct Workspaces {
    inner: Ring<Workspace>,
    pub(super) previous_workspace: usize,
    client_insert_point: InsertPoint,
    main_ratio_step: f32,
}

impl Deref for Workspaces {
    type Target = Ring<Workspace>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Workspaces {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Workspaces {
    pub fn new(workspaces: Vec<Workspace>, main_ratio_step: f32) -> Self {
        Self {
            inner: Ring::new(workspaces),
            previous_workspace: 0,
            client_insert_point: InsertPoint::First,
            main_ratio_step,
        }
    }

    pub fn get_workspace(&self, ix: usize) -> Result<&Workspace> {
        self.inner
            .get(ix)
            .ok_or_else(|| perror!("unknown workspace: {}", ix))
    }

    pub fn would_focus(&self, ix: usize, selector: &Selector<'_, Workspace>) -> bool {
        self.inner
            .equivalent_selectors(&Selector::Index(ix), selector)
    }

    pub fn workspace(&self, selector: &Selector<'_, Workspace>) -> Option<&Workspace> {
        if let Selector::WinId(id) = selector {
            self.inner.iter().find(|ws| ws.client_ids().contains(&id))
        } else {
            self.inner.element(&selector)
        }
    }

    pub fn workspace_mut(&mut self, selector: &Selector<'_, Workspace>) -> Option<&mut Workspace> {
        if let Selector::WinId(id) = selector {
            self.inner
                .iter_mut()
                .find(|ws| ws.client_ids().contains(&id))
        } else {
            self.inner.element_mut(&selector)
        }
    }

    pub fn matching_workspaces(&self, selector: &Selector<'_, Workspace>) -> Vec<&Workspace> {
        if let Selector::WinId(id) = selector {
            self.inner
                .iter()
                .find(|ws| ws.client_ids().contains(&id))
                .into_iter()
                .collect()
        } else {
            self.inner.all_elements(&selector)
        }
    }

    pub fn matching_workspaces_mut(
        &mut self,
        selector: &Selector<'_, Workspace>,
    ) -> Vec<&mut Workspace> {
        if let Selector::WinId(id) = selector {
            self.inner
                .iter_mut()
                .find(|ws| ws.client_ids().contains(&id))
                .into_iter()
                .collect()
        } else {
            self.inner.all_elements_mut(&selector)
        }
    }

    pub fn workspace_names(&self) -> Vec<String> {
        self.inner.iter().map(|ws| ws.name().to_string()).collect()
    }

    pub fn set_workspace_name(
        &mut self,
        name: impl Into<String>,
        selector: &Selector<'_, Workspace>,
    ) {
        let s = name.into();
        self.inner.apply_to(selector, |ws| {
            ws.set_name(&s);
        });
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn add_client(&mut self, wix: usize, id: Xid) -> Result<Option<EventAction>> {
        if let Some(ws) = self.inner.get_mut(wix) {
            ws.add_client(id, &self.client_insert_point)?;
            Ok(Some(EventAction::RunHook(
                HookName::ClientAddedToWorkspace(id, wix),
            )))
        } else {
            Ok(None)
        }
    }

    pub fn remove_client(&mut self, wix: usize, id: Xid) {
        self.inner.apply_to(&Selector::Index(wix), |ws| {
            ws.remove_client(id);
        });
    }

    pub fn add_workspace(&mut self, ix: usize, ws: Workspace) {
        self.inner.insert(ix, ws);
    }

    pub fn push_workspace(&mut self, ws: Workspace) {
        self.inner.push(ws);
    }

    pub fn remove_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Result<Workspace> {
        self.inner
            .remove(&selector)
            .ok_or_else(|| perror!("unknown workspace"))
    }

    pub fn set_client_insert_point(&mut self, cip: InsertPoint) {
        self.client_insert_point = cip;
    }

    pub fn get_arrange_actions(
        &mut self,
        wix: usize,
        region: Region,
        clients: &[&Client],
    ) -> Result<(LayoutConf, ArrangeActions)> {
        let ws = self
            .inner
            .get(wix)
            .ok_or_else(|| perror!("attempt to layout unknown workspace: {}", wix))?;

        let lc = ws.layout_conf();
        if !lc.floating {
            Ok((lc, ws.arrange(region, clients)))
        } else {
            Ok((
                lc,
                ArrangeActions {
                    actions: vec![],
                    floating: clients.iter().map(|c| c.id()).collect(),
                },
            ))
        }
    }

    pub fn cycle_workspace(&mut self, direction: Direction) -> usize {
        self.inner.cycle_focus(direction);
        self.inner.focused_index()
    }

    pub fn cycle_client(&mut self, wix: usize, direction: Direction) -> Option<(Xid, Xid)> {
        self.inner
            .get_mut(wix)
            .and_then(|ws| ws.cycle_client(direction))
    }

    pub fn drag_client(&mut self, wix: usize, direction: Direction) {
        self.inner.apply_to(&Selector::Index(wix), |ws| {
            ws.drag_client(direction);
        });
    }

    pub fn rotate_clients(&mut self, wix: usize, direction: Direction) {
        self.inner.apply_to(&Selector::Index(wix), |ws| {
            ws.rotate_clients(direction);
        });
    }

    pub fn cycle_layout(&mut self, wix: usize, direction: Direction) {
        self.inner.apply_to(&Selector::Index(wix), |ws| {
            ws.cycle_layout(direction);
        });
    }

    pub fn update_max_main(&mut self, wix: usize, change: Change) {
        self.inner.apply_to(&Selector::Index(wix), |ws| {
            ws.update_max_main(change);
        });
    }

    pub fn update_main_ratio(&mut self, wix: usize, change: Change) {
        let step = self.main_ratio_step;
        self.inner.apply_to(&Selector::Index(wix), |ws| {
            ws.update_main_ratio(change, step);
        });
    }

    pub fn current_layout_symbol(&self, wix: usize) -> &str {
        match self.inner.get(wix) {
            Some(ws) => ws.layout_symbol(),
            None => "???",
        }
    }

    pub fn client_ids(&self, wix: usize) -> Result<Vec<Xid>> {
        self.inner
            .get(wix)
            .map(|ws| ws.client_ids())
            .ok_or_else(|| perror!("unknown workspace: {}", wix))
    }

    pub fn focused_client(&self, ix: usize) -> Option<Xid> {
        self.inner[ix].focused_client()
    }

    #[cfg(feature = "serde")]
    pub fn restore_layout_functions(
        &mut self,
        layout_funcs: &HashMap<&str, LayoutFunc>,
    ) -> Result<()> {
        self.inner
            .iter_mut()
            .try_for_each(|ws| ws.restore_layout_functions(&layout_funcs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::__test_helpers::*;

    // 9 empty workspaces
    fn workspaces() -> Workspaces {
        Workspaces::new(
            (1..10)
                .map(|ix| test_workspace(format!("{}", ix), 0))
                .collect(),
            0.1,
        )
    }

    #[test]
    fn add_workspace() {
        let mut wss = workspaces();

        wss.add_workspace(1, test_workspace("new", 0));
        assert_eq!(
            wss.workspace_names(),
            vec!["1", "new", "2", "3", "4", "5", "6", "7", "8", "9"]
        );
    }

    #[test]
    fn push_workspace() {
        let mut wss = workspaces();

        wss.push_workspace(test_workspace("new", 0));
        assert_eq!(
            wss.workspace_names(),
            vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "new"]
        );
    }

    #[test]
    fn remove_workspace() {
        let mut wss = workspaces();

        let removed = wss.remove_workspace(&Selector::Index(2)).unwrap();
        assert_eq!(removed.name(), "3");
        assert_eq!(
            wss.workspace_names(),
            vec!["1", "2", "4", "5", "6", "7", "8", "9"]
        );
    }

    #[test]
    fn remove_workspace_unknown_is_error() {
        let mut wss = workspaces();

        let res = wss.remove_workspace(&Selector::Index(42));
        assert!(res.is_err())
    }

    // Full tests of Ring::insert are handled in ring.rs
    // This is just to validate that Workspaces honours the insert point being set
    #[test]
    fn set_client_insert_point() {
        let mut wss = Workspaces::new(vec![test_workspace("test", 2)], 0.1);
        assert_eq!(wss[0].client_ids(), vec![0, 1]);

        wss.set_client_insert_point(InsertPoint::First);
        wss.add_client(0, 2).unwrap();
        assert_eq!(wss[0].client_ids(), vec![2, 0, 1]);

        wss.set_client_insert_point(InsertPoint::Last);
        wss.add_client(0, 3).unwrap();
        assert_eq!(wss[0].client_ids(), vec![2, 0, 1, 3]);
    }

    #[test]
    fn add_duplicate_client_is_error() {
        let mut wss = Workspaces::new(vec![test_workspace("test", 1)], 0.1);
        assert_eq!(wss[0].client_ids(), vec![0]);

        let res = wss.add_client(0, 0);
        assert!(res.is_err());
    }
}
