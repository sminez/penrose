//! Metadata around X clients and manipulating them
use crate::core::data_types::WinId;

/**
 * Meta-data around a client window that we are handling.
 *
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Clone)]
pub struct Client {
    id: WinId,
    wm_name: String,
    wm_class: String,
    workspace: usize,
    // state flags
    pub(crate) floating: bool,
    pub(crate) fullscreen: bool,
    pub(crate) mapped: bool,
    pub(crate) wm_managed: bool,
}

impl Client {
    /// Track a new client window on a specific workspace
    pub(crate) fn new(
        id: WinId,
        wm_name: String,
        wm_class: String,
        workspace: usize,
        floating: bool,
    ) -> Client {
        Client {
            id,
            wm_name,
            wm_class,
            workspace,
            floating,
            fullscreen: false,
            mapped: false,
            wm_managed: true,
        }
    }

    /// The X window ID of this client
    pub fn id(&self) -> WinId {
        self.id
    }

    /// The WM_CLASS property of this client
    pub fn wm_class(&self) -> &str {
        &self.wm_class
    }

    /// The WM_NAME property of this client
    pub fn wm_name(&self) -> &str {
        &self.wm_name
    }

    /// The current workspace index that this client is showing on
    pub fn workspace(&self) -> usize {
        self.workspace
    }

    /// Mark this window as being on a new workspace
    pub fn set_workspace(&mut self, workspace: usize) {
        self.workspace = workspace
    }

    pub(crate) fn set_name(&mut self, name: impl Into<String>) {
        self.wm_name = name.into()
    }

    /// The WM_CLASS of the window that this Client is tracking
    pub fn class(&self) -> &str {
        &self.wm_class
    }

    /// Mark this client as not being managed by the WindowManager directly
    pub fn externally_managed(&mut self) {
        self.wm_managed = false;
    }

    /// Mark this client as being managed by the WindowManager directly
    pub fn internally_managed(&mut self) {
        self.wm_managed = true;
    }
}
