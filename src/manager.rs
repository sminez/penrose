use crate::client::Client;
use crate::config;
use crate::data_types::{KeyBindings, KeyCode};
use crate::helpers::{grab_keys, str_prop};
use crate::screen::Screen;
use crate::workspace::Workspace;
use std::process;
use xcb;

// pulling out bitmasks to make the following xcb / xrandr calls easier to parse visually
const NEW_WINDOW_MASK: &[(u32, u32)] = &[(
    xcb::CW_EVENT_MASK,
    xcb::EVENT_MASK_ENTER_WINDOW | xcb::EVENT_MASK_LEAVE_WINDOW,
)];
const WIN_X: u16 = xcb::CONFIG_WINDOW_X as u16;
const WIN_Y: u16 = xcb::CONFIG_WINDOW_Y as u16;
const WIN_WIDTH: u16 = xcb::CONFIG_WINDOW_WIDTH as u16;
const WIN_HEIGHT: u16 = xcb::CONFIG_WINDOW_HEIGHT as u16;
// const WIN_BORDER: u16 = xcb::CONFIG_WINDOW_BORDER_WIDTH as u16;

/**
 * WindowManager is the primary struct / owner of the event loop ofr penrose.
 * It handles most (if not all) of the communication with XCB and responds to
 * X events served over the embedded connection. User input bindings are parsed
 * and bound on init and then triggered via grabbed X events in the main loop
 * along with everything else.
 */
pub struct WindowManager {
    conn: xcb::Connection,
    screens: Vec<Screen>,
    workspaces: Vec<Workspace>,
    clients: Vec<Client>,
    focused_screen: usize,
}

impl WindowManager {
    pub fn init() -> WindowManager {
        let (mut conn, _) = match xcb::Connection::connect(None) {
            Err(e) => die!("unable to establish connection to X server: {}", e),
            Ok(conn) => conn,
        };
        let screens = Screen::current_outputs(&mut conn);

        WindowManager {
            conn,
            screens,
            workspaces: config::WORKSPACES
                .iter()
                .map(|name| Workspace::new(name, config::layouts()))
                .collect(),
            clients: vec![],
            focused_screen: 0,
        }
    }

    fn apply_layout(&self, screen: usize) {
        let screen_region = self.screens[screen].region;
        let ws = self.workspace(screen);

        for (id, region) in ws.arrange(&screen_region) {
            log!("configuring {} with {:?}", id, region);
            let (x, y, w, h) = region.values();
            let padding = 2 * (config::BORDER_PX + config::GAP_PX);

            xcb::configure_window(
                &self.conn,
                id,
                &[
                    (WIN_X, x as u32 + config::GAP_PX),
                    (WIN_Y, y as u32 + config::GAP_PX),
                    (WIN_WIDTH, w as u32 - padding),
                    (WIN_HEIGHT, h as u32 - padding),
                ],
            );
        }
    }

    // xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    // fn button_press(&mut self, event: &xcb::ButtonPressEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    // fn button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_input_device_key_press_event_t
    fn key_press(&mut self, event: &xcb::KeyPressEvent, bindings: &KeyBindings) {
        log!("handling keypress: {} {}", event.state(), event.detail());

        if let Some(action) = bindings.get(&KeyCode::from_key_press(event)) {
            action(self);
        }
    }

    // xcb docs: https://www.mankier.com/3/xcb_xkb_map_notify_event_t
    fn new_window(&mut self, event: &xcb::MapNotifyEvent) {
        let win_id = event.window();
        let wm_class = match str_prop(&self.conn, win_id, "WM_CLASS") {
            Ok(s) => s.split("\0").collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        };
        // let window_type = atom_prop(&self.conn, window, "_NET_WM_WINDOW_TYPE");
        log!("handling new window: {}", wm_class);

        let floating = config::FLOATING_CLASSES.contains(&wm_class.as_ref());
        let client = Client::new(win_id, wm_class, 1, floating);
        self.clients.push(client);

        if !floating {
            self.workspace_mut(self.focused_screen).add_client(win_id);
        }

        log!("currently have {} known clients", self.clients.len());

        // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
        xcb::change_window_attributes(&self.conn, win_id, NEW_WINDOW_MASK);

        self.apply_layout(self.focused_screen);
    }

    // xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    // fn focus_window(&mut self, event: &xcb::EnterNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    // fn unfocus_window(&mut self, event: &xcb::LeaveNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_motion_notify_event_t
    // fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_destroy_notify_event_t
    fn destroy_window(&mut self, event: &xcb::DestroyNotifyEvent) {
        let win_id = event.window();

        log!("removing ref to win_id {}", win_id);

        self.workspace_mut(self.focused_screen)
            .remove_client(win_id);
        self.clients.retain(|c| c.id != win_id);
        self.apply_layout(0);
    }

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn run(&mut self) {
        let bindings = config::key_bindings();
        grab_keys(&self.conn, &bindings);

        loop {
            if let Some(event) = self.conn.wait_for_event() {
                match event.response_type() {
                    // user input
                    xcb::KEY_PRESS => self.key_press(unsafe { xcb::cast_event(&event) }, &bindings),
                    // xcb::BUTTON_PRESS => self.button_press(unsafe { xcb::cast_event(&event) }),
                    // xcb::BUTTON_RELEASE => self.button_release(unsafe { xcb::cast_event(&event) }),
                    // window actions
                    xcb::MAP_NOTIFY => self.new_window(unsafe { xcb::cast_event(&event) }),
                    // xcb::ENTER_NOTIFY => self.focus_window(unsafe { xcb::cast_event(&event) }),
                    // xcb::LEAVE_NOTIFY => self.unfocus_window(unsafe { xcb::cast_event(&event) }),
                    // xcb::MOTION_NOTIFY => self.resize_window(unsafe { xcb::cast_event(&event) }),
                    xcb::DESTROY_NOTIFY => self.destroy_window(unsafe { xcb::cast_event(&event) }),
                    // unknown event type
                    _ => (),
                }
            }

            self.conn.flush();
        }
    }

    fn workspace(&self, screen_index: usize) -> &Workspace {
        &self.workspaces[self.screens[screen_index].wix]
    }

    fn workspace_mut(&mut self, screen_index: usize) -> &mut Workspace {
        &mut self.workspaces[self.screens[screen_index].wix]
    }

    // Public methods that can be triggered by user bindings

    /// Exit the main event loop and perform cleanup
    pub fn kill(&mut self) {
        // TODO: ungrab keys? need to check what cleanup needs to be done
        self.conn.flush();
        process::exit(0);
    }

    /// Set the displayed tag for the current screen
    pub fn set_tag(&mut self, tag: usize) {
        log!("setting tag: {}", tag);
    }

    /// Add an additional tag to the current screen
    pub fn add_tag(&mut self, tag: usize) {
        log!("adding tag {}", tag);
    }

    /// Set the tag for the currently highlighted client
    pub fn tag_client(&mut self, tag: usize) {
        log!("tagging client: {}", tag);
    }

    /// Move the next available layout, forward or backwards through the stack
    pub fn cycle_layout(&mut self, forward: bool) {
        log!("cycling layout: {}", forward);
    }

    pub fn inc_main(&mut self) {
        self.workspace_mut(self.focused_screen).inc_main();
        self.apply_layout(self.focused_screen);
    }

    pub fn dec_main(&mut self) {
        self.workspace_mut(self.focused_screen).dec_main();
        self.apply_layout(self.focused_screen);
    }

    pub fn inc_ratio(&mut self) {
        self.workspace_mut(self.focused_screen).inc_ratio();
        self.apply_layout(self.focused_screen);
    }

    pub fn dec_ratio(&mut self) {
        self.workspace_mut(self.focused_screen).dec_ratio();
        self.apply_layout(self.focused_screen);
    }
}
