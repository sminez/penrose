use crate::{
    core::{
        client::Client,
        data_types::{Region, WinId},
        layout::LayoutConf,
        screen::Screen,
        workspace::{ArrangeActions, Workspace},
        xconnection::{Atom, XConn},
    },
    Result,
};

#[cfg(feature = "serde")]
use crate::{core::manager::WindowManager, PenroseError};

use std::collections::HashMap;

pub(super) struct ClientProps {
    pub(super) name: String,
    pub(super) class: String,
    pub(super) ty: String,
}

pub(super) fn pad_region(region: &Region, gapless: bool, gap_px: u32, border_px: u32) -> Region {
    let gpx = if gapless { 0 } else { gap_px };
    let padding = 2 * (border_px + gpx);
    let (x, y, w, h) = region.values();
    Region::new(x + gpx, y + gpx, w - padding, h - padding)
}

pub(super) fn map_window_if_needed<X: XConn>(conn: &X, win: Option<&mut Client>) {
    if let Some(c) = win {
        if !c.mapped {
            c.mapped = true;
            conn.map_window(c.id());
        }
    }
}

pub(super) fn unmap_window_if_needed<X: XConn>(conn: &X, win: Option<&mut Client>) {
    if let Some(c) = win {
        if c.mapped {
            c.mapped = false;
            conn.unmap_window(c.id());
        }
    }
}

pub(super) fn client_str_props<X: XConn>(conn: &X, id: WinId) -> ClientProps {
    ClientProps {
        name: match conn.str_prop(id, Atom::WmName.as_ref()) {
            Ok(s) => s,
            Err(_) => String::from("n/a"),
        },
        class: match conn.str_prop(id, Atom::WmClass.as_ref()) {
            Ok(s) => s.split('\0').collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        },
        ty: match conn.str_prop(id, Atom::NetWmWindowType.as_ref()) {
            Ok(s) => s.split('\0').collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        },
    }
}

pub(super) fn position_floating_client<X: XConn>(
    conn: &X,
    id: WinId,
    screen_region: Region,
    gap_px: u32,
    border_px: u32,
) -> Result<()> {
    let default_position = conn.window_geometry(id)?;
    let (mut x, mut y, w, h) = default_position.values();
    let (sx, sy, _, _) = screen_region.values();
    x = if x < sx { sx } else { x };
    y = if y < sy { sy } else { y };
    let reg = Region::new(x, y, w, h);
    let reg = pad_region(&reg, false, gap_px, border_px);
    conn.position_window(id, reg, border_px, false);

    Ok(())
}

pub(super) fn window_name<X: XConn>(conn: &X, id: WinId) -> Result<String> {
    match conn.str_prop(id, Atom::NetWmName.as_ref()) {
        Ok(s) if !s.is_empty() => Ok(s),
        _ => conn.str_prop(id, Atom::WmName.as_ref()),
    }
}

pub(super) fn get_screens<X: XConn>(
    conn: &X,
    mut visible_workspaces: Vec<usize>,
    n_workspaces: usize,
    bar_height: u32,
    top_bar: bool,
) -> Vec<Screen> {
    // Keeping the currently displayed workspaces on the active screens if possible and then
    // filling in with remaining workspaces in ascending order
    visible_workspaces.append(
        &mut (0..n_workspaces)
            .filter(|w| !visible_workspaces.contains(w))
            .collect(),
    );
    debug!("Current workspace ordering: {:?}", visible_workspaces);
    conn.current_outputs()
        .into_iter()
        .zip(visible_workspaces)
        .map(|(mut s, wix)| {
            s.update_effective_region(bar_height, top_bar);
            debug!("Setting focused workspace for screen {:?}", s);
            s.wix = wix;
            info!("Detected Screen: {:?}", s);
            s
        })
        .collect()
}

pub(super) fn toggle_fullscreen<X: XConn>(
    conn: &X,
    id: WinId,
    client_map: &mut HashMap<WinId, Client>,
    workspace: &mut Workspace,
    screen_size: Region,
) -> bool {
    if !client_map.contains_key(&id) {
        warn!("unable to make unknown client fullscreen: {}", id);
        return false;
    }
    let client_currently_fullscreen = client_map.get(&id).map(|c| c.fullscreen).unwrap();
    conn.toggle_client_fullscreen(id, client_currently_fullscreen);

    workspace.client_ids().iter_mut().for_each(|&mut i| {
        if client_currently_fullscreen {
            if i == id {
                client_map.entry(id).and_modify(|c| c.fullscreen = false);
            } else {
                map_window_if_needed(conn, client_map.get_mut(&i));
            }
        // client was not fullscreen
        } else if i == id {
            conn.position_window(id, screen_size, 0, false);
            client_map.entry(id).and_modify(|c| {
                map_window_if_needed(conn, Some(c));
                c.fullscreen = true;
            });
        } else {
            unmap_window_if_needed(conn, client_map.get_mut(&i));
        }
    });

    // need to apply layout if true as we just came back from being fullscreen and
    // there are newly mapped windows that need to be laid out
    client_currently_fullscreen
}

pub(super) fn apply_arrange_actions<X: XConn>(
    conn: &X,
    actions: ArrangeActions,
    lc: &LayoutConf,
    client_map: &mut HashMap<WinId, Client>,
    border_px: u32,
    gap_px: u32,
) {
    // Tile first then place floating clients on top
    for (id, region) in actions.actions {
        let possible_client = client_map.get_mut(&id);
        debug!("configuring {} with {:?}", id, region);
        if let Some(region) = region {
            let reg = pad_region(&region, lc.gapless, gap_px, border_px);
            conn.position_window(id, reg, border_px, false);
            map_window_if_needed(conn, possible_client);
        } else {
            unmap_window_if_needed(conn, possible_client);
        }
    }

    for id in actions.floating {
        conn.raise_window(id);
    }
}

#[cfg(feature = "serde")]
pub(super) fn validate_hydrated_wm_state<X: XConn>(wm: &mut WindowManager<X>) -> Result<()> {
    // If the current clients known to the X server aren't what we have in the client_map
    // then we can't proceed any further
    let active_windows = wm.conn.query_for_active_windows();
    let mut missing_ids: Vec<WinId> = wm
        .client_map
        .keys()
        .filter(|id| !active_windows.contains(id))
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
    use crate::{
        core::{
            layout::{mock_layout, Layout, LayoutConf},
            ring::InsertPoint,
            workspace::Workspace,
            xconnection::StubXConn,
        },
        PenroseError,
    };

    use std::{cell::Cell, collections::HashMap, str::FromStr};

    #[test]
    fn pad_region_centered() {
        let r = Region::new(0, 0, 200, 100);
        let g = 10;
        let b = 3;
        assert_eq!(pad_region(&r, false, g, b), Region::new(10, 10, 174, 74));
        assert_eq!(pad_region(&r, true, g, b), Region::new(0, 0, 194, 94));
    }

    struct WmNameXConn {
        wm_name: bool,
        net_wm_name: bool,
        empty_net_wm_name: bool,
    }
    impl StubXConn for WmNameXConn {
        fn mock_str_prop(&self, _: WinId, name: &str) -> Result<String> {
            match Atom::from_str(name)? {
                Atom::WmName if self.wm_name => Ok("wm_name".into()),
                Atom::WmName if self.net_wm_name && self.empty_net_wm_name => Ok("".into()),
                Atom::NetWmName if self.net_wm_name => Ok("net_wm_name".into()),
                Atom::NetWmName if self.empty_net_wm_name => Ok("".into()),
                _ => Err(PenroseError::Raw("".into())),
            }
        }
    }

    test_cases! {
        window_name;
        args: (wm_name: bool, net_wm_name: bool, empty_net_wm_name: bool, expected: &str);

        case: wm_name_only => (true, false, false, "wm_name");
        case: net_wm_name_only => (false, true, false, "net_wm_name");
        case: both_prefers_net => (true, true, false, "net_wm_name");
        case: net_wm_name_empty => (true, false, true, "wm_name");

        body: {
            let conn = WmNameXConn {
                wm_name,
                net_wm_name,
                empty_net_wm_name,
            };
            assert_eq!(&window_name(&conn, 42).unwrap(), expected);
        }
    }

    struct OutputsXConn(Vec<Screen>);
    impl StubXConn for OutputsXConn {
        fn mock_current_outputs(&self) -> Vec<Screen> {
            self.0.clone()
        }
    }

    fn test_screens(h: u32, top_bar: bool) -> Vec<Screen> {
        let regions = &[
            Region::new(0, 0, 1000, 800),
            Region::new(1000, 0, 1400, 900),
        ];
        regions
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                let mut s = Screen::new(r, i);
                s.update_effective_region(h, top_bar);
                s
            })
            .collect()
    }

    test_cases! {
        get_screens;
        args: (current: Vec<usize>, n_workspaces: usize, expected: Vec<usize>);

        case: unchanged => (vec![0, 1], 10, vec![0, 1]);
        case: non_default_workspaces => (vec![5, 7], 10, vec![5, 7]);
        case: new_take_first_available_0 => (vec![0], 10, vec![0, 1]);
        case: new_take_first_available_2 => (vec![2], 10, vec![2, 0]);
        case: fewer_retains_from_left => (vec![3, 5, 9], 10, vec![3, 5]);
        case: more_truncates => (vec![0], 1, vec![0]);

        body: {
            let (bar_height, top_bar) = (10, true);
            let screens = test_screens(bar_height, top_bar);
            let conn = OutputsXConn(screens);
            let new = get_screens(&conn, current, n_workspaces, bar_height, top_bar);
            let focused: Vec<usize> = new.iter().map(|s| s.wix).collect();

            assert_eq!(focused, expected);
        }
    }

    struct RecordingXConn {
        positions: Cell<Vec<(WinId, Region)>>,
        maps: Cell<Vec<WinId>>,
        unmaps: Cell<Vec<WinId>>,
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

    impl StubXConn for RecordingXConn {
        fn mock_position_window(&self, id: WinId, r: Region, _: u32, _: bool) {
            let mut v = self.positions.take();
            v.push((id, r));
            self.positions.set(v);
        }

        fn mock_map_window(&self, id: WinId) {
            let mut v = self.maps.take();
            v.push(id);
            self.maps.set(v);
        }

        fn mock_unmap_window(&self, id: WinId) {
            let mut v = self.unmaps.take();
            v.push(id);
            self.unmaps.set(v);
        }
    }

    test_cases! {
        toggle_fullscreen;
        args: (
            n_clients: usize,
            fullscreen: Option<WinId>,
            target: WinId,
            unmapped: &[WinId],
            expected_need_layout: bool,
            expected_positions: Vec<WinId>,
            expected_maps: Vec<WinId>,
            expected_unmaps: Vec<WinId>,
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
                    let mut client = Client::new(id, "name".into(), "class".into(), 0, false);
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

            assert_eq!(need_layout, expected_need_layout);
            assert_eq!(conn.positions.take(), expected_positions);
            assert_eq!(conn.maps.take(), expected_maps);
            assert_eq!(conn.unmaps.take(), expected_unmaps);
        }
    }
}
