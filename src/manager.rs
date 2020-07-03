use crate::config;
use crate::data_types::{KeyBindings, Region};
use std::process;
use xcb;

pub struct WindowManager {
    conn: xcb::Connection,
    screen_num: i32,
    screen_dims: Vec<Region>,
    screen_tags: Vec<usize>,
    key_bindings: KeyBindings,
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
        };

        wm.update_screen_dimensions();
        wm.grab_keys();
        wm.screen_tags = wm.screen_dims.iter().enumerate().map(|(i, _)| i).collect();

        wm
    }

    fn grab_keys(&self) {
        // pulling out bitmasks to make the following xcb / xrandr calls easier to parse visually
        let notify_mask = xcb::randr::NOTIFY_MASK_CRTC_CHANGE as u16;
        let mode = xcb::GRAB_MODE_ASYNC as u8;
        let event_mask = &[(
            xcb::CW_EVENT_MASK,
            xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32,
        )];
        let mouse_mask = (xcb::EVENT_MASK_BUTTON_PRESS
            | xcb::EVENT_MASK_BUTTON_RELEASE
            | xcb::EVENT_MASK_POINTER_MOTION) as u16;

        let screen = self.conn.get_setup().roots().nth(0).unwrap();
        let root = screen.root();

        let input = xcb::randr::select_input(&self.conn, root, notify_mask);
        match input.request_check() {
            Err(e) => die!("randr error: {}", e),
            Ok(_) => {
                for k in self.key_bindings.keys() {
                    xcb::grab_key(&self.conn, false, root, k.mask, k.code, mode, mode);
                }
            }
        }

        for mouse_button in &[1, 3] {
            xcb::grab_button(
                &self.conn,
                false,
                root,
                mouse_mask,
                mode,
                mode,
                xcb::NONE,
                xcb::NONE,
                *mouse_button,
                xcb::MOD_MASK_4 as u16,
            );
        }

        xcb::change_window_attributes(&self.conn, root, event_mask);
        self.conn.flush();
    }

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

    fn button_press(&mut self, event: &xcb::ButtonPressEvent) {}
    fn button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}
    fn key_press(&mut self, event: &xcb::KeyPressEvent) {}
    fn new_window(&mut self, event: &xcb::MapNotifyEvent) {}
    fn focus_window(&mut self, event: &xcb::EnterNotifyEvent) {}
    fn unfocus_window(&mut self, event: &xcb::LeaveNotifyEvent) {}
    fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) {}
    fn destroy_window(&mut self, event: &xcb::DestroyNotifyEvent) {}

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn run(&mut self) {
        loop {
            if let Some(event) = self.conn.wait_for_event() {
                match event.response_type() {
                    // user input
                    xcb::KEY_PRESS => self.key_press(unsafe { xcb::cast_event(&event) }),
                    xcb::BUTTON_PRESS => self.button_press(unsafe { xcb::cast_event(&event) }),
                    xcb::BUTTON_RELEASE => self.button_release(unsafe { xcb::cast_event(&event) }),
                    // window actions
                    xcb::MAP_NOTIFY => self.new_window(unsafe { xcb::cast_event(&event) }),
                    xcb::ENTER_NOTIFY => self.focus_window(unsafe { xcb::cast_event(&event) }),
                    xcb::LEAVE_NOTIFY => self.unfocus_window(unsafe { xcb::cast_event(&event) }),
                    xcb::MOTION_NOTIFY => self.resize_window(unsafe { xcb::cast_event(&event) }),
                    xcb::DESTROY_NOTIFY => self.destroy_window(unsafe { xcb::cast_event(&event) }),
                    // unknown event type
                    _ => (),
                }
            }

            self.conn.flush();
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
