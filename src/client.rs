use crate::config;
use crate::data_types::Region;

/**
 * Meta-data around a client window that we are handling.
 * Primarily state flags and information used when determining which clients
 * to show for a given monitor and how they are tiled.
 */
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Client {
    id: u32,
    tag: u32,
    region: Region,
    border_width: u32,
    // state flags
    is_urgent: bool,
    is_focused: bool,
    is_floating: bool,
    is_fullscreen: bool,
}

impl Client {
    pub fn width_on_resize(&self, r: Region) -> u32 {
        return r.width() + 2 * self.border_width + config::GAP_PX;
    }

    pub fn height_on_resize(&self, r: Region) -> u32 {
        return r.height() + 2 * self.border_width + config::GAP_PX;
    }

    pub fn is_tiled_for_tag(&self, mask: u32) -> bool {
        !self.is_floating && (mask & self.tag > 0)
    }
}
