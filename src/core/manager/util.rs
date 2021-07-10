use crate::core::{
    client::Client,
    data_types::Region,
    layout::LayoutConf,
    workspace::{ArrangeActions, Workspace},
    xconnection::{XClientConfig, XClientHandler, XClientProperties, XState, Xid},
};

#[cfg(feature = "serde")]
use crate::{
    core::{manager::WindowManager, xconnection::XConn},
    PenroseError,
};

use std::collections::HashMap;

pub(super) fn pad_region(region: &Region, gapless: bool, gap_px: u32, border_px: u32) -> Region {
    let gpx = if gapless { 0 } else { gap_px };
    let padding = 2 * (border_px + gpx);
    let (x, y, w, h) = region.values();
    Region::new(x + gpx, y + gpx, w - padding, h - padding)
}

pub(super) fn position_floating_client<X>(
    conn: &X,
    id: Xid,
    screen_region: Region,
    border_px: u32,
) -> crate::Result<()>
where
    X: XClientConfig + XState,
{
    let default_position = conn.client_geometry(id)?;
    let (mut x, mut y, w, h) = default_position.values();
    let (sx, sy, _, _) = screen_region.values();
    x = if x < sx { sx } else { x };
    y = if y < sy { sy } else { y };
    let reg = Region::new(
        x + border_px,
        y + border_px,
        w - (2 * border_px),
        h - (2 * border_px),
    );

    Ok(conn.position_client(id, reg, border_px, false)?)
}

pub(super) fn toggle_fullscreen<X>(
    conn: &X,
    id: Xid,
    client_map: &mut HashMap<Xid, Client>,
    workspace: &mut Workspace,
    screen_size: Region,
) -> crate::Result<bool>
where
    X: XClientHandler + XClientProperties + XClientConfig,
{
    if !client_map.contains_key(&id) {
        warn!(id, "attempt to make unknown client fullscreen");
        return Ok(false);
    }
    let client_currently_fullscreen = client_map.get(&id).map(|c| c.fullscreen).unwrap();
    conn.toggle_client_fullscreen(id, client_currently_fullscreen)?;

    for i in workspace.client_ids().into_iter() {
        if client_currently_fullscreen {
            if i == id {
                client_map.entry(id).and_modify(|c| c.fullscreen = false);
            } else {
                conn.map_client_if_needed(client_map.get_mut(&i))?;
            }
        // client was not fullscreen
        } else if i == id {
            conn.position_client(id, screen_size, 0, false)?;
            if let Some(c) = client_map.get_mut(&id) {
                conn.map_client_if_needed(Some(c))?;
                c.fullscreen = true;
            }
        } else {
            conn.unmap_client_if_needed(client_map.get_mut(&i))?;
        }
    }

    // need to apply layout if true as we just came back from being fullscreen and
    // there are newly mapped windows that need to be laid out
    Ok(client_currently_fullscreen)
}

pub(super) fn apply_arrange_actions<X>(
    conn: &X,
    actions: ArrangeActions,
    lc: &LayoutConf,
    client_map: &mut HashMap<Xid, Client>,
    border_px: u32,
    gap_px: u32,
) -> crate::Result<()>
where
    X: XClientHandler + XClientConfig,
{
    // Tile first then place floating clients on top
    for (id, region) in actions.actions {
        let possible_client = client_map.get_mut(&id);
        debug!(id, ?region, "positioning client");
        if let Some(region) = region {
            let reg = pad_region(&region, lc.gapless, gap_px, border_px);
            conn.position_client(id, reg, border_px, false)?;
            conn.map_client_if_needed(possible_client)?;
        } else {
            conn.unmap_client_if_needed(possible_client)?;
        }
    }

    for id in actions.floating {
        debug!(id, "mapping floating client above tiled");
        conn.raise_client(id)?;
    }

    Ok(())
}

#[cfg(feature = "serde")]
pub(super) fn validate_hydrated_wm_state<X>(wm: &mut WindowManager<X>) -> crate::Result<()>
where
    X: XConn,
{
    // If the current clients known to the X server aren't what we have in the client_map
    // then we can't proceed any further
    let active_clients = wm.conn.active_clients()?;
    let mut missing_ids: Vec<Xid> = wm
        .client_map
        .keys()
        .filter(|id| !active_clients.contains(id))
        .cloned()
        .collect();

    if !missing_ids.is_empty() {
        missing_ids.sort_unstable();
        return Err(PenroseError::MissingClientIds(missing_ids));
    }

    // Workspace clients are all need to be present in the client_map
    wm.workspaces.iter().try_for_each(|w| {
        if w.iter().all(|id| wm.client_map.contains_key(id)) {
            Ok(())
        } else {
            Err(PenroseError::HydrationState(
                "one or more workspace clients we not in known client state".into(),
            ))
        }
    })?;

    // If current focused client is not in the client_map then it was most likely being
    // managed by a user defined hook.
    if let Some(id) = wm.focused_client {
        if !wm.client_map.contains_key(&id) {
            wm.focused_client = None;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        layout::{mock_layout, Layout, LayoutConf},
        ring::InsertPoint,
        workspace::Workspace,
        xconnection::*,
    };

    use std::{cell::Cell, collections::HashMap};

    #[test]
    fn pad_region_centered() {
        let r = Region::new(0, 0, 200, 100);
        let g = 10;
        let b = 3;
        assert_eq!(pad_region(&r, false, g, b), Region::new(10, 10, 174, 74));
        assert_eq!(pad_region(&r, true, g, b), Region::new(0, 0, 194, 94));
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
        fn mock_map_client(&self, id: Xid) -> Result<()> {
            let mut v = self.maps.take();
            v.push(id);
            self.maps.set(v);
            Ok(())
        }

        fn mock_unmap_client(&self, id: Xid) -> Result<()> {
            let mut v = self.unmaps.take();
            v.push(id);
            self.unmaps.set(v);
            Ok(())
        }
    }

    impl StubXClientConfig for RecordingXConn {
        fn mock_position_client(&self, id: Xid, r: Region, _: u32, _: bool) -> Result<()> {
            let mut v = self.positions.take();
            v.push((id, r));
            self.positions.set(v);
            Ok(())
        }
    }

    test_cases! {
        toggle_fullscreen;
        args: (
            n_clients: usize,
            fullscreen: Option<Xid>,
            target: Xid,
            unmapped: &[Xid],
            expected_need_layout: bool,
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
            let mut ws = Workspace::new(
                "test",
                vec![Layout::new("t", LayoutConf::default(), mock_layout, 1, 0.6)],
            );
            let mut client_map: HashMap<_, _> = (0..n_clients)
                .map(|id| {
                    let id = id as u32;
                    let mut client = Client::new(&conn, id, 0, &[]);
                    client.mapped = true;
                    ws.add_client(id, &InsertPoint::Last).unwrap();
                    (id, client)
                })
                .collect();

            let r = Region::new(0, 0, 1000, 800);
            let expected_positions: Vec<_> = expected_positions.iter().map(|id| (*id, r)).collect();

            for id in unmapped {
                client_map.entry(*id).and_modify(|c| c.mapped = false);
            }

            if let Some(id) = fullscreen {
                client_map.entry(id).and_modify(|c| c.fullscreen = true);
            }

            let need_layout = toggle_fullscreen(&conn, target, &mut client_map, &mut ws, r);

            assert_eq!(need_layout.unwrap(), expected_need_layout);
            assert_eq!(conn.positions.take(), expected_positions);
            assert_eq!(conn.maps.take(), expected_maps);
            assert_eq!(conn.unmaps.take(), expected_unmaps);
        }
    }
}
