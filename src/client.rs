use crate::config;
use crate::data_types::WinId;

/**
 * Meta-data around a client window that we are handling.
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[derive(Debug, PartialEq, Clone)]
pub struct Client {
    pub id: WinId,
    wm_class: String,
    tag: u32,
    border_width: u32,
    // state flags
    pub is_urgent: bool,
    pub is_focused: bool,
    pub is_floating: bool,
    pub is_fullscreen: bool,
}

impl Client {
    pub fn new(id: WinId, wm_class: String, tag: u32, floating: bool) -> Client {
        Client {
            id,
            wm_class,
            tag,
            border_width: config::BORDER_PX,
            is_urgent: false,
            is_focused: true,
            is_floating: floating,
            is_fullscreen: false,
        }
    }

    pub fn is_tiled_for_tag(&self, mask: u32) -> bool {
        !self.is_floating && (mask & self.tag > 0)
    }
}
