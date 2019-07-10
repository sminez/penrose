extern crate x11;

use x11::{xlib};


pub struct Monitor {
    id: i32,  // num
    layout_symbol: String,  // ltsymbol
    master_ratio: f32,  // mfact
    n_master: i32,
    bar_height: i32,  // by
    screen_size: Region,
    window_area: Region,
    selected_tags: u8,
    selected_layout: u8,
    tagset: [u8; 2],
    show_bar: bool,
    top_bar: bool,
    client_list: &ClientList,
    selected_client: &Client,
    next_monitor: &Monitor,
    bar_window: xlib::Window,
    layout: *Layout,
}
