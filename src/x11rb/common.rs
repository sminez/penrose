use crate::{
    core::{
        data_types::{PropVal, Region, WinId},
        screen::Screen,
        xconnection::Atom,
    },
    x11rb::Result as X11Result,
};

use x11rb::{
    connection::Connection,
    protocol::{
        randr::ConnectionExt as _,
        xproto::{AtomEnum, ConnectionExt as _, PropMode},
    },
    wrapper::ConnectionExt as _,
};

use strum::IntoEnumIterator;

use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct Atoms {
    atoms: HashMap<Atom, u32>,
}

impl Atoms {
    pub(crate) fn new(conn: &impl Connection) -> X11Result<Self> {
        let atoms = Atom::iter()
            .map(|atom| Ok((atom, conn.intern_atom(false, atom.as_ref().as_bytes())?)))
            .collect::<X11Result<Vec<_>>>()?;
        let atoms = atoms.into_iter()
            .map(|(atom, cookie)| Ok((atom, cookie.reply()?.atom)))
            .collect::<X11Result<HashMap<_, _>>>()?;
        Ok(Self { atoms })
    }

    pub(crate) fn known_atom(&self, atom: Atom) -> u32 {
        *self.atoms.get(&atom).unwrap()
    }
}

pub(crate) fn current_screens(conn: &impl Connection, win: WinId) -> X11Result<Vec<Screen>> {
    let resources = conn.randr_get_screen_resources(win)?.reply()?;
    // Send queries for all CRTCs
    let crtcs = resources.crtcs.iter()
        .map(|c| conn.randr_get_crtc_info(*c, 0).map_err(|err| err.into()))
        .collect::<X11Result<Vec<_>>>()?;
    // Get the replies and construct screens
    let screens = crtcs.into_iter()
        .flat_map(|cookie| cookie.reply().ok())
        .enumerate()
        .filter(|(_, reply)| reply.width > 0)
        .map(|(i, reply)| {
            let region = Region::new(
                reply.x as u32,
                reply.y as u32,
                reply.width as u32,
                reply.height as u32,
            );
            Screen::new(region, i)
        })
        .collect();
    Ok(screens)
}

pub(crate) fn replace_prop(conn: &impl Connection, win: WinId, prop: u32, val: PropVal<'_>) -> X11Result<()> {
    let (kind, data) = match val {
        PropVal::Atom(data) => (AtomEnum::ATOM, data),
        PropVal::Cardinal(data) => (AtomEnum::CARDINAL, data),
        PropVal::Window(data) => (AtomEnum::WINDOW, data),
        PropVal::Str(s) => {
            conn.change_property8(PropMode::REPLACE, win, prop, AtomEnum::STRING, s.as_bytes())?;
            return Ok(())
        }
    };
    conn.change_property32(PropMode::REPLACE, win, prop, kind, data)?;
    Ok(())
}
