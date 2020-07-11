//! Metadata around X clients and manipulating them
use crate::data_types::WinId;

/**
 * Meta-data around a client window that we are handling.
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[derive(Debug, PartialEq, Clone)]
pub struct Client {
    id: WinId,
    wm_class: String,
    workspace: usize,
    // state flags
    floating: bool,
    fullscreen: bool,
}

impl Client {
    pub fn new(id: WinId, wm_class: String, workspace: usize, floating: bool) -> Client {
        Client {
            id,
            wm_class,
            workspace,
            floating: floating,
            fullscreen: false,
        }
    }

    pub fn id(&self) -> WinId {
        self.id
    }

    /// The current workspace index that this client is showing on
    pub fn workspace(&self) -> usize {
        self.workspace
    }

    pub fn set_workspace(&mut self, workspace: usize) {
        self.workspace = workspace
    }

    pub fn class(&self) -> &str {
        &self.wm_class
    }
}
