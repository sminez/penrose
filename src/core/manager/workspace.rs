use crate::core::{
    client::Client,
    data_types::{Region, WinId},
    manager::util::{map_window_if_needed, pad_region, unmap_window_if_needed},
    workspace::Workspace,
    xconnection::XConn,
};

use std::collections::HashMap;

pub(super) fn apply_layout(
    conn: &Box<dyn XConn>,
    workspace: &Workspace,
    reg: Region,
    client_map: &mut HashMap<WinId, Client>,
    border_px: u32,
    gap_px: u32,
) {
    let lc = workspace.layout_conf();
    if !lc.floating {
        let arrange_result = workspace.arrange(reg, client_map);

        // Tile first then place floating clients on top
        for (id, region) in arrange_result.actions {
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

        for id in arrange_result.floating {
            conn.raise_window(id);
        }
    }
}
