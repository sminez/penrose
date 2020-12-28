use crate::{
    core::{
        client::Client,
        data_types::{Region, WinId},
        xconnection::{Atom, XConn},
    },
    Result,
};

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
) {
    if let Ok(default_position) = conn.window_geometry(id) {
        let (mut x, mut y, w, h) = default_position.values();
        let (sx, sy, _, _) = screen_region.values();
        x = if x < sx { sx } else { x };
        y = if y < sy { sy } else { y };
        let reg = Region::new(x, y, w, h);
        let reg = pad_region(&reg, false, gap_px, border_px);
        conn.position_window(id, reg, border_px, false);
    }
}

pub(super) fn window_name(conn: &Box<dyn XConn>, id: WinId) -> Result<String> {
    conn.str_prop(id, Atom::WmName.as_ref())
        .or_else(|_| conn.str_prop(id, Atom::NetWmName.as_ref()))
}
