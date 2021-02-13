use crate::{core::xconnection::Atom, x11rb::Result};

use std::collections::HashMap;

use strum::IntoEnumIterator;

use x11rb::{connection::Connection, protocol::xproto::ConnectionExt};

#[derive(Debug)]
pub(crate) struct Atoms {
    atoms: HashMap<Atom, u32>,
}

impl Atoms {
    pub(crate) fn new(conn: &impl Connection) -> Result<Self> {
        // First send all requests...
        let atom_requests = Atom::iter()
            .map(|atom| Ok((atom, conn.intern_atom(false, atom.as_ref().as_bytes())?)))
            .collect::<Result<Vec<_>>>()?;
        // ..then get all the replies (so that we only need one instead of many round-trips to the
        // X11 server)
        let atoms = atom_requests
            .into_iter()
            .map(|(atom, cookie)| Ok((atom, cookie.reply()?.atom)))
            .collect::<Result<HashMap<_, _>>>()?;
        Ok(Self { atoms })
    }

    pub(crate) fn known_atom(&self, atom: Atom) -> u32 {
        *self.atoms.get(&atom).unwrap()
    }

    pub(crate) fn atom_name(&self, atom: u32) -> Option<Atom> {
        self.atoms
            .iter()
            .find(|(_, value)| atom == **value)
            .map(|(key, _)| *key)
    }
}
