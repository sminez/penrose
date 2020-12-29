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

use std::collections::HashMap;

pub(super) struct ClientProps {
    pub(super) name: String,
    pub(super) class: String,
    pub(super) ty: String,
}

pub(super) fn vec_string_to_str(v: &[String]) -> Vec<&str> {
    v.iter().map(|s| s.as_ref()).collect()
}

pub(super) fn pad_region(region: &Region, gapless: bool, gap_px: u32, border_px: u32) -> Region {
    let gpx = if gapless { 0 } else { gap_px };
    let padding = 2 * (border_px + gpx);
    let (x, y, w, h) = region.values();
    Region::new(x + gpx, y + gpx, w - padding, h - padding)
}

pub(super) fn map_window_if_needed(conn: &Box<dyn XConn>, win: Option<&mut Client>) {
    if let Some(c) = win {
        if !c.mapped {
            c.mapped = true;
            conn.map_window(c.id());
        }
    }
}

pub(super) fn unmap_window_if_needed(conn: &Box<dyn XConn>, win: Option<&mut Client>) {
    if let Some(c) = win {
        if c.mapped {
            c.mapped = false;
            conn.unmap_window(c.id());
        }
    }
}

pub(super) fn client_str_props(conn: &Box<dyn XConn>, id: WinId) -> ClientProps {
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

pub(super) fn position_floating_client(
    conn: &Box<dyn XConn>,
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

pub(super) fn window_name(conn: &Box<dyn XConn>, id: WinId) -> Result<String> {
    conn.str_prop(id, Atom::NetWmName.as_ref())
        .or_else(|_| conn.str_prop(id, Atom::WmName.as_ref()))
}

pub(super) fn get_screens(
    conn: &Box<dyn XConn>,
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

pub(super) fn toggle_fullscreen(
    conn: &Box<dyn XConn>,
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

    workspace.clients().iter_mut().for_each(|&mut i| {
        if client_currently_fullscreen {
            if i == id {
                client_map.entry(id).and_modify(|c| c.fullscreen = false);
            } else {
                map_window_if_needed(&conn, client_map.get_mut(&i));
            }
        // client was not fullscreen
        } else if i == id {
            conn.position_window(id, screen_size, 0, false);
            client_map.entry(id).and_modify(|c| {
                map_window_if_needed(conn, Some(c));
                c.fullscreen = true;
            });
        } else {
            unmap_window_if_needed(&conn, client_map.get_mut(&i));
        }
    });

    // need to apply layout if true as we just came back from being fullscreen and
    // there are newly mapped windows that need to be laid out
    client_currently_fullscreen
}

pub(super) fn apply_arrange_actions(
    conn: &Box<dyn XConn>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        layout::{mock_layout, Layout, LayoutConf},
        ring::InsertPoint,
        workspace::Workspace,
        xconnection::StubXConn,
    };

    use anyhow::anyhow;
    use test_case::test_case;

    use std::{cell::RefCell, collections::HashMap, rc::Rc, str::FromStr};

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
    }
    impl StubXConn for WmNameXConn {
        fn mock_str_prop(&self, _: WinId, name: &str) -> Result<String> {
            match Atom::from_str(name)? {
                Atom::WmName if self.wm_name => Ok("wm_name".into()),
                Atom::NetWmName if self.net_wm_name => Ok("net_wm_name".into()),
                _ => Err(anyhow!("")),
            }
        }
    }

    #[test_case(true, false, "wm_name"; "wm_name only")]
    #[test_case(false, true, "net_wm_name"; "net_wm_name only")]
    #[test_case(true, true, "net_wm_name"; "both prefers net_wm_name")]
    fn window_name_test(wm_name: bool, net_wm_name: bool, expected: &str) {
        let conn = Box::new(WmNameXConn {
            wm_name,
            net_wm_name,
        }) as Box<dyn XConn>;
        assert_eq!(&window_name(&conn, 42).unwrap(), expected);
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

    #[test_case(vec![0, 1], 10, vec![0, 1]; "unchanged")]
    #[test_case(vec![5, 7], 10, vec![5, 7]; "non default workspaces")]
    #[test_case(vec![0], 10, vec![0, 1]; "new screens take first available 0")]
    #[test_case(vec![2], 10, vec![2, 0]; "new screens take first available 2")]
    #[test_case(vec![3, 5, 9], 10, vec![3, 5]; "fewer screens reatin from the left")]
    #[test_case(vec![0], 1, vec![0]; "more screens that workspaces truncates")]
    fn get_screens_test(current: Vec<usize>, n_workspaces: usize, expected: Vec<usize>) {
        let (bar_height, top_bar) = (10, true);
        let screens = test_screens(bar_height, top_bar);
        let conn = Box::new(OutputsXConn(screens)) as Box<dyn XConn>;
        let new = get_screens(&conn, current, n_workspaces, bar_height, top_bar);
        let focused: Vec<usize> = new.iter().map(|s| s.wix).collect();

        assert_eq!(focused, expected);
    }

    type RRV<T> = Rc<RefCell<Vec<T>>>;

    struct RecordingXConn {
        positions: RRV<(WinId, Region)>,
        maps: RRV<WinId>,
        unmaps: RRV<WinId>,
    }
    impl RecordingXConn {
        #[allow(clippy::type_complexity)]
        fn init() -> (RRV<(WinId, Region)>, RRV<WinId>, RRV<WinId>, Box<dyn XConn>) {
            let positions = Rc::new(RefCell::new(vec![]));
            let maps = Rc::new(RefCell::new(vec![]));
            let unmaps = Rc::new(RefCell::new(vec![]));
            let conn = Box::new(Self {
                positions: Rc::clone(&positions),
                maps: Rc::clone(&maps),
                unmaps: Rc::clone(&unmaps),
            });

            (positions, maps, unmaps, conn)
        }
    }
    impl StubXConn for RecordingXConn {
        fn mock_position_window(&self, id: WinId, r: Region, _: u32, _: bool) {
            self.positions.replace_with(|v| {
                v.push((id, r));
                v.to_vec()
            });
        }

        fn mock_map_window(&self, id: WinId) {
            self.maps.replace_with(|v| {
                v.push(id);
                v.to_vec()
            });
        }

        fn mock_unmap_window(&self, id: WinId) {
            self.unmaps.replace_with(|v| {
                v.push(id);
                v.to_vec()
            });
        }
    }

    fn strip<T>(r: RRV<T>) -> Vec<T> {
        match Rc::try_unwrap(r) {
            Ok(inner) => inner.into_inner(),
            Err(_) => panic!("shouldn't have outstanding refs at this point"),
        }
    }

    #[test_case(1, None, 0, &[], false, vec![0], vec![], vec![]; "single client on")]
    #[test_case(1, Some(0), 0, &[], true, vec![], vec![], vec![]; "single client off")]
    #[test_case(4, None, 1, &[], false, vec![1], vec![], vec![0, 2, 3]; "multiple clients on")]
    #[test_case(4, Some(1), 1, &[0, 2, 3], true, vec![], vec![0, 2, 3], vec![]; "multiple clients off")]
    fn toggle_fullscreen_test(
        n_clients: usize,
        fullscreen: Option<WinId>,
        target: WinId,
        unmapped: &[WinId],
        expected_need_layout: bool,
        expected_positions: Vec<WinId>,
        expected_maps: Vec<WinId>,
        expected_unmaps: Vec<WinId>,
    ) {
        let (positions, maps, unmaps, conn) = RecordingXConn::init();
        let mut ws = Workspace::new(
            "test",
            vec![Layout::new("t", LayoutConf::default(), mock_layout, 1, 0.6)],
        );
        let mut client_map: HashMap<_, _> = (0..n_clients)
            .map(|id| {
                let id = id as u32;
                let mut client = Client::new(id, "name".into(), "class".into(), 0, false);
                client.mapped = true;
                ws.add_client(id, &InsertPoint::Last);
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
        drop(conn);

        assert_eq!(need_layout, expected_need_layout);
        assert_eq!(strip(positions), expected_positions);
        assert_eq!(strip(maps), expected_maps);
        assert_eq!(strip(unmaps), expected_unmaps);
    }
}
