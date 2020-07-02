use crate::config;
use crate::monitor::Monitor;
use crate::util::Region;
use x11::xlib;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Client {
    // name: String,
    tags: usize,

    pub x_window: xlib::Window,
    pub region: Region,
    pub old_region: Region,

    min_alpha: f32,
    max_alpha: f32,

    pub base_width: usize,
    pub max_width: usize,
    pub min_width: usize,
    pub inc_width: usize,

    pub base_height: usize,
    pub max_height: usize,
    pub min_height: usize,
    pub inc_height: usize,

    pub border_width: usize,
    old_border_width: usize,

    pub is_fixed: bool,
    pub is_floating: bool,
    pub is_urgent: bool,
    pub never_focus: bool,
    pub old_state: bool,
    pub is_fullscreen: bool,
    pub is_pinned: bool,
}

impl Client {
    pub fn width_on_resize(&self, r: Region) -> usize {
        return r.w + 2 * self.border_width + config::GAP_PX;
    }

    pub fn height_on_resize(&self, r: Region) -> usize {
        return r.h + 2 * self.border_width + config::GAP_PX;
    }

    pub fn is_tiled_on_monitor(&self, m: &Monitor) -> bool {
        !self.is_floating && (self.is_pinned || m.is_showing_tag(self.tags))
    }

    pub fn configure(&mut self) {}
}
