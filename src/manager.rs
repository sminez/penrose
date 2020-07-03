use crate::config;
use crate::data_types::{KeyBindings, Region};
use crate::helpers::str_prop;
use std::process;
use xcb;

// pulling out bitmasks to make the following xcb / xrandr calls easier to parse visually
const NOTIFY_MASK: u16 = xcb::randr::NOTIFY_MASK_CRTC_CHANGE as u16;
const GRAB_MODE_ASYNC: u8 = xcb::GRAB_MODE_ASYNC as u8;
const EVENT_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY as u32,
)];
const MOUSE_MASK: u16 = (xcb::EVENT_MASK_BUTTON_PRESS
    | xcb::EVENT_MASK_BUTTON_RELEASE
    | xcb::EVENT_MASK_POINTER_MOTION) as u16;

/**
 * WindowManager is the primary struct / owner of the event loop ofr penrose.
 * It handles most (if not all) of the communication with XCB and responds to
 * X events served over the embedded connection. User input bindings are parsed
 * and bound on init and then triggered via grabbed X events in the main loop
 * along with everything else.
 */
pub struct WindowManager {
    conn: xcb::Connection,
    screen_num: i32,
    screen_dims: Vec<Region>,
    screen_tags: Vec<usize>,
    key_bindings: KeyBindings,
}

impl WindowManager {
    pub fn init() -> WindowManager {
        let (conn, screen_num) = xcb::Connection::connect(None).unwrap();

        let mut wm = WindowManager {
            conn,
            screen_num,
            screen_dims: vec![],
            screen_tags: vec![],
            key_bindings: config::key_bindings(),
        };

        wm.screen_tags = wm.screen_dims.iter().enumerate().map(|(i, _)| i).collect();
        wm.update_screen_dimensions();
        wm.grab_keys();

        wm
    }

    fn grab_keys(&self) {
        let screen = self.conn.get_setup().roots().nth(0).unwrap();
        let root = screen.root();

        // xcb docs: https://www.mankier.com/3/xcb_randr_select_input
        let input = xcb::randr::select_input(&self.conn, root, NOTIFY_MASK);
        match input.request_check() {
            Err(e) => die!("randr error: {}", e),
            Ok(_) => {
                for k in self.key_bindings.keys() {
                    // xcb docs: https://www.mankier.com/3/xcb_grab_key
                    xcb::grab_key(
                        &self.conn,      // xcb connection to X11
                        false,           // don't pass grabbed events through to the client
                        root,            // the window to grab: in this case the root window
                        k.mask,          // modifiers to grab
                        k.code,          // keycode to grab
                        GRAB_MODE_ASYNC, // don't lock pointer input while grabbing
                        GRAB_MODE_ASYNC, // don't lock keyboard input while grabbing
                    );
                }
            }
        }

        for mouse_button in &[1, 3] {
            // xcb docs: https://www.mankier.com/3/xcb_grab_button
            xcb::grab_button(
                &self.conn,             // xcb connection to X11
                false,                  // don't pass grabbed events through to the client
                root,                   // the window to grab: in this case the root window
                MOUSE_MASK,             // which events are reported to the client
                GRAB_MODE_ASYNC,        // don't lock pointer input while grabbing
                GRAB_MODE_ASYNC,        // don't lock keyboard input while grabbing
                xcb::NONE,              // don't confine the cursor to a specific window
                xcb::NONE,              // don't change the cursor type
                *mouse_button,          // the button to grab
                xcb::MOD_MASK_4 as u16, // modifiers to grab
            );
        }

        // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
        xcb::change_window_attributes(&self.conn, root, EVENT_MASK);
        self.conn.flush();
    }

    fn update_screen_dimensions(&mut self) {
        let screen = match self.conn.get_setup().roots().nth(0) {
            None => die!("unable to get handle for screen"),
            Some(s) => s,
        };

        let win_id = self.conn.generate_id();
        let root = screen.root();

        // xcb docs: https://www.mankier.com/3/xcb_create_window
        xcb::create_window(
            &self.conn, // xcb connection to X11
            0,          // new window's depth
            win_id,     // ID to be used for referring to the window
            root,       // parent window
            0,          // x-coordinate
            0,          // y-coordinate
            1,          // width
            1,          // height
            0,          // border width
            0,          // class (i _think_ 0 == COPY_FROM_PARENT?)
            0,          // visual (i _think_ 0 == COPY_FROM_PARENT?)
            &[],        // value list? (value mask? not documented either way...)
        );

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_screen_resources
        let resources = xcb::randr::get_screen_resources(&self.conn, win_id);

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_crtc_info
        self.screen_dims = match resources.get_reply() {
            Err(e) => die!("error reading X screen resources: {}", e),
            Ok(reply) => reply
                .crtcs()
                .iter()
                .flat_map(|c| xcb::randr::get_crtc_info(&self.conn, *c, 0).get_reply())
                .map(|r| Region::from_crtc_info_reply(r))
                .filter(|r| r.width() > 0)
                .collect(),
        };
    }

    // xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    fn button_press(&mut self, event: &xcb::ButtonPressEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    fn button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_input_device_key_press_event_t
    fn key_press(&mut self, event: &xcb::KeyPressEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_xkb_map_notify_event_t
    fn new_window(&mut self, event: &xcb::MapNotifyEvent) {
        let wm_class = str_prop(&self.conn, event.window(), "WM_CLASS");
    }

    // xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    fn focus_window(&mut self, event: &xcb::EnterNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    fn unfocus_window(&mut self, event: &xcb::LeaveNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_motion_notify_event_t
    fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_destroy_notify_event_t
    fn destroy_window(&mut self, event: &xcb::DestroyNotifyEvent) {}

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn run(&mut self) {
        let setup = self.conn.get_setup();
        let screen = setup.roots().nth(self.screen_num as usize).unwrap();

        println!("penrose screen details {}:", screen.root());
        println!("  width..........: {}", screen.width_in_pixels());
        println!("  height.........: {}", screen.height_in_pixels());
        println!("  white pixel....: {:x}", screen.white_pixel());
        println!("  black pixel....: {:x}", screen.black_pixel());

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

    /// Exit the main event loop and perform cleanup
    pub fn kill(&mut self) {
        // TODO: ungrab keys? need to check what cleanup needs to be done
        self.conn.flush();
        process::exit(0);
    }

    /// Set the displayed tag for the current screen
    pub fn set_tag(&mut self, tag: usize) {}

    /// Add an additional tag to the current screen
    pub fn add_tag(&mut self, tag: usize) {}

    /// Set the tag for the currently highlighted client
    pub fn tag_client(&mut self, tag: usize) {}

    /// Move the next available layout, forward or backwards through the stack
    pub fn cycle_layout(&mut self, forward: bool) {}
}
