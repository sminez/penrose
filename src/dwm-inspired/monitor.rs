use crate::client::Client;
use crate::layout::{Layout, LayoutKind, ResizeAction};
use crate::util::Region;
use x11::xlib;

#[derive(Clone)]
pub struct Monitor {
    id: usize,                   // num
    layout_symbol: &'static str, // ltsymbol
    bar_height: usize,           // by
    pub screen_region: Region,
    pub window_region: Region,
    tag_mask: usize,
    bar_window: xlib::Window,
    layouts: Vec<Layout>,
    current_layout: usize,
}

impl Monitor {
    pub fn is_showing_tag(&self, tag: usize) -> bool {
        self.tag_mask & tag > 0
    }

    pub fn get_layout_actions(&self, clients: Vec<Client>) -> Vec<ResizeAction> {
        let (for_mon, not_for_mon): (Vec<Client>, Vec<Client>) =
            clients.iter().partition(|&c| c.is_tiled_on_monitor(self));

        let mut actions = self.layouts[self.current_layout].arrange(
            for_mon
                .into_iter()
                .filter(|c| c.is_tiled_on_monitor(self))
                .collect(),
            &self.window_region,
        );

        actions.append(
            &mut not_for_mon
                .iter()
                .map(|&c| ResizeAction { c, r: None })
                .collect(),
        );

        actions
    }

    pub fn layout_kind(&self) -> LayoutKind {
        self.layouts[self.current_layout].kind
    }
}
