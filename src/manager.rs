use crate::config;
use crate::data_types::{CodeMap, KeyCode, Region};
use crate::helpers::keycodes_from_xmodmap;
use std::collections::HashMap;
use std::process;
use xcb;

pub type FireAndForget = Box<dyn Fn(&mut WindowManager) -> ()>;
pub type KeyBindings = HashMap<KeyCode, FireAndForget>;

pub struct WindowManager {
    conn: xcb::Connection,
    screen_num: i32,
    screen_dims: Vec<Region>,
    screen_tags: Vec<usize>,
    key_bindings: KeyBindings,
    xmodmap_codes: CodeMap,
}

impl WindowManager {
    pub fn new() -> WindowManager {
        let (conn, screen_num) = xcb::Connection::connect(None).unwrap();

        let mut wm = WindowManager {
            conn,
            screen_num,
            screen_dims: vec![],
            screen_tags: vec![],
            key_bindings: config::key_bindings(),
            xmodmap_codes: keycodes_from_xmodmap(),
        };

        wm.update_screen_dimensions();
        wm.grab_keys();
        wm.screen_tags = wm.screen_dims.iter().enumerate().map(|(i, _)| i).collect();

        wm
    }

    fn grab_keys(&mut self) {}

    fn update_screen_dimensions(&mut self) {
        let screen = match self.conn.get_setup().roots().nth(0) {
            None => die!("unable to get handle for screen"),
            Some(s) => s,
        };

        let win_id = self.conn.generate_id();
        let root = screen.root();

        // TODO: add a comment on what the args for this are
        xcb::create_window(&self.conn, 0, win_id, root, 0, 0, 1, 1, 0, 0, 0, &[]);
        let resources = xcb::randr::get_screen_resources(&self.conn, win_id);

        // TODO: add a comment on what this is doing
        self.screen_dims = match resources.get_reply() {
            Err(e) => die!("error reading X screen resources: {}", e),
            Ok(reply) => reply
                .crtcs()
                .iter()
                .flat_map(|c| xcb::randr::get_crtc_info(&self.conn, *c, 0).get_reply())
                .map(|r| Region {
                    x: r.x() as usize,
                    y: r.y() as usize,
                    w: r.width() as usize,
                    h: r.height() as usize,
                })
                .filter(|r| r.w > 0)
                .collect(),
        };
    }

    pub fn run(&mut self) {
        println!("{} keys bound", self.key_bindings.len());
        for key in self.key_bindings.keys() {
            println!("{:?}", key);
        }
    }

    // Public methods that can be triggered by user bindings

    pub fn kill(&mut self) {
        println!("This may be disgusting, but it works...");
        process::exit(0);
    }

    pub fn set_tag(&mut self, tag: usize) {}
    pub fn add_tag(&mut self, tag: usize) {}
    pub fn tag_client(&mut self, tag: usize) {}
}
