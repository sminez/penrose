use crate::layout::Layout;
use crate::util::Region;
use x11::xlib;

#[derive(Clone)]
pub struct Monitor<'a> {
    id: usize,                   // num
    layout_symbol: &'static str, // ltsymbol
    bar_height: usize,           // by
    pub screen_region: Region,
    pub window_region: Region,
    tag_mask: usize,
    bar_window: xlib::Window,
    layouts: Vec<Layout<'a>>,
    current_layout: usize,
}

impl<'a> Monitor<'a> {
    pub fn is_showing_tag(&self, tag: usize) -> bool {
        self.tag_mask & tag > 0
    }

    pub fn layout(&self) -> &'a Layout<'_> {
        &self.layouts[self.current_layout]
    }
}
