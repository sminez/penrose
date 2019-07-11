extern crate x11;

use crate::client::{Client, ClientList};
use crate::layouts::Layout;
use crate::util::Region;
use x11::xlib;

pub struct Monitor<'a> {
    id: i32,               // num
    layout_symbol: String, // ltsymbol
    master_ratio: f32,     // mfact
    n_master: i32,
    bar_height: i32, // by
    screen_size: Region,
    window_area: Region,
    selected_tags: u8,
    selected_layout: u8,
    tagset: [u8; 2],
    show_bar: bool,
    top_bar: bool,
    pub client_list: &'a mut ClientList<'a>,
    selected_client: &'a Client<'a>,
    next_monitor: &'a Monitor<'a>,
    bar_window: xlib::Window,
    layout: &'a Layout,
}

impl<'a> Monitor<'a> {
    pub fn update_Bar_position(&mut self, bar_height: i32) {
        self.window_area.y = self.screen_size.y;
        self.window_area.h = self.screen_size.h;

        if self.show_bar {
            self.screen_size.h -= bar_height;
        // m->by = m->topbar ? m->wy : m->wy + m->wh;
        // m->wy = m->topbar ? m->wy + bh : m->wy;
        } else {
            self.bar_height -= bar_height;
        }
    }
}
