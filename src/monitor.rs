use crate::client::Client;
use crate::layout::Layout;
use crate::util::Region;
use x11::xlib;

pub struct Monitor<'a> {
    id: usize,             // num
    layout_symbol: String, // ltsymbol
    pub master_ratio: f32, // mfact
    pub n_master: usize,
    bar_height: usize, // by
    pub screen_region: Region,
    pub window_region: Region,
    tags: usize,
    // selected_tags: usize,
    // tagset: [u8; 2],
    selected_layout: usize,
    pub client_list: Vec<Client<'a>>,
    selected_client: usize, // index into above
    bar_window: xlib::Window,
    pub layout: &'a dyn Layout<'a>,
}

impl<'a> Monitor<'a> {
    pub fn n_clients(&self) -> usize {
        self.client_list.len()
    }

    pub fn n_tiled_clients(&self) -> usize {
        // TODO: don't include floating or invisible clients
        self.client_list.iter().filter(|c| !c.is_floating).count()
    }

    pub fn is_showing_tag(&self, tags: usize) -> bool {
        self.tags & tags > 0
    }
}
