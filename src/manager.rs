use crate::client::Client;
use crate::data_types::{Change, ColorScheme, Config, Direction, KeyBindings, KeyCode, WinId};
use crate::helpers::{grab_keys, intern_atom, str_prop};
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
const WIN_BORDER: u16 = xcb::CONFIG_WINDOW_BORDER_WIDTH as u16;

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
    focused_screen: usize,
    // config
    fonts: &'static [&'static str],
    floating_classes: &'static [&'static str],
    color_scheme: ColorScheme,
    border_px: u32,
    gap_px: u32,
    main_ratio_step: f32,
    systray_spacing_px: u32,
    show_systray: bool,
    show_bar: bool,
    top_bar: bool,
    respect_resize_hints: bool,
}

impl WindowManager {
    pub fn init(conf: Config) -> WindowManager {
        let (mut conn, _) = match xcb::Connection::connect(None) {
            Err(e) => die!("unable to establish connection to X server: {}", e),
            Ok(conn) => conn,
        };
        let screens = Screen::current_outputs(&mut conn);
        log!("connected to X server: {} screens detected", screens.len());

        let workspaces: Vec<Workspace> = conf
            .workspaces
            .iter()
            .map(|name| Workspace::new(name, conf.layouts.clone()))
            .collect();

        WindowManager {
            conn,
            screens,
            workspaces,
            focused_screen: 0,
            fonts: conf.fonts,
            floating_classes: conf.floating_classes,
            color_scheme: conf.color_scheme,
            border_px: conf.border_px,
            gap_px: conf.gap_px,
            main_ratio_step: conf.main_ratio_step,
            systray_spacing_px: conf.systray_spacing_px,
            show_systray: conf.show_systray,
            show_bar: conf.show_bar,
            top_bar: conf.top_bar,
            respect_resize_hints: conf.respect_resize_hints,
        }
    }

    fn apply_layout(&self, screen: usize) {
        let screen_region = self.screens[screen].region;
        let ws = self.workspace_for_screen(screen);

        for (id, region) in ws.arrange(&screen_region) {
            debug!("configuring {} with {:?}", id, region);
            let (x, y, w, h) = region.values();
            let padding = 2 * (self.border_px + self.gap_px);

            xcb::configure_window(
                &self.conn,
                id,
                &[
                    (WIN_X, x as u32 + self.gap_px),
                    (WIN_Y, y as u32 + self.gap_px),
                    (WIN_WIDTH, w as u32 - padding),
                    (WIN_HEIGHT, h as u32 - padding),
                    (WIN_BORDER, self.border_px),
                ],
            );
        }
    }

    fn remove_client(&mut self, win_id: WinId) {
        debug!("removing ref to client {}", win_id);

        // TODO: may not be on this screen...
        self.workspace_for_screen_mut(self.focused_screen)
            .remove_client(win_id);
    }

    // xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    // fn button_press(&mut self, event: &xcb::ButtonPressEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_input_raw_button_press_event_t
    // fn button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_input_device_key_press_event_t
    fn key_press(&mut self, event: &xcb::KeyPressEvent, bindings: &KeyBindings) {
        debug!("handling keypress: {} {}", event.state(), event.detail());
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

        debug!("handling new window: {}", wm_class);
        let floating = self.floating_classes.contains(&wm_class.as_ref());
        let client = Client::new(win_id, wm_class, floating, self.border_px);

        if !floating {
            self.workspace_for_screen_mut(self.focused_screen)
                .add_client(client);
        }

        // xcb docs: https://www.mankier.com/3/xcb_change_window_attributes
        xcb::change_window_attributes(&self.conn, win_id, NEW_WINDOW_MASK);
        self.apply_layout(self.focused_screen);
    }

    // xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    fn focus_window(&mut self, event: &xcb::EnterNotifyEvent) {
        let win_id = event.event();
        debug!("focusing client {}", win_id);
        for ws in self.workspaces.iter_mut() {
            ws.focus_client(win_id, &self.conn, &self.color_scheme);
        }
    }

    // xcb docs: https://www.mankier.com/3/xcb_enter_notify_event_t
    fn unfocus_window(&mut self, event: &xcb::LeaveNotifyEvent) {
        let win_id = event.event();
        for ws in self.workspaces.iter_mut() {
            match ws.focused_client_mut() {
                Some(client) => {
                    if client.id == win_id {
                        client.unfocus(&self.conn, &self.color_scheme);
                        return;
                    }
                }
                None => (),
            }
        }
    }

    // xcb docs: https://www.mankier.com/3/xcb_motion_notify_event_t
    // fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) {}

    // xcb docs: https://www.mankier.com/3/xcb_destroy_notify_event_t
    fn destroy_window(&mut self, event: &xcb::DestroyNotifyEvent) {
        self.remove_client(event.window());
        self.apply_layout(self.focused_screen);
    }

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn grab_keys_and_run(&mut self, bindings: KeyBindings) {
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
                    xcb::ENTER_NOTIFY => self.focus_window(unsafe { xcb::cast_event(&event) }),
                    xcb::LEAVE_NOTIFY => self.unfocus_window(unsafe { xcb::cast_event(&event) }),
                    // xcb::MOTION_NOTIFY => self.resize_window(unsafe { xcb::cast_event(&event) }),
                    xcb::DESTROY_NOTIFY => self.destroy_window(unsafe { xcb::cast_event(&event) }),
                    // unknown event type
                    _ => (),
                }
            }

            self.conn.flush();
        }
    }

    fn workspace_for_screen(&self, screen_index: usize) -> &Workspace {
        &self.workspaces[self.screens[screen_index].wix]
    }

    fn workspace_for_screen_mut(&mut self, screen_index: usize) -> &mut Workspace {
        &mut self.workspaces[self.screens[screen_index].wix]
    }

    fn focused_client(&self) -> Option<&Client> {
        self.workspace_for_screen(self.focused_screen)
            .focused_client()
    }

    fn cycle_client(&mut self, direction: Direction) {
        let scheme = self.color_scheme.clone();
        self.workspaces[self.screens[self.focused_screen].wix]
            .cycle_client(direction, &self.conn, &scheme);
    }

    /*
     * Public methods that can be triggered by user bindings
     *
     * User defined hooks can be implemented by adding additional logic to these
     * handlers which will then be run each time they are triggered
     */

    pub fn exit(&mut self) {
        self.conn.flush();
        process::exit(0);
    }

    pub fn switch_workspace(&mut self, index: usize) {
        notify!("switching to ws: {}", index);
        match index {
            0 => run_external!("xsetroot -solid #282828")(self),
            1 => run_external!("xsetroot -solid #cc241d")(self),
            2 => run_external!("xsetroot -solid #458588")(self),
            3 => run_external!("xsetroot -solid #fabd2f")(self),
            4 => run_external!("xsetroot -solid #b8bb26")(self),
            _ => run_external!("xsetroot -solid #ebdbb2")(self),
        };

        for i in 0..self.screens.len() {
            if self.screens[i].wix == index {
                if i == self.focused_screen {
                    return; // already focused on the current screen
                }

                // The workspace we want is currently displayed on another screen so
                // pull the target workspace to the focused screen, and place the
                // workspace we had on the screen where the target was
                self.screens[i].wix = self.screens[self.focused_screen].wix;
                self.screens[self.focused_screen].wix = index;
                self.apply_layout(self.focused_screen);
                self.apply_layout(i);
                return;
            }
        }

        // target not currently displayed
        let current = self.screens[self.focused_screen].wix;
        self.screens[self.focused_screen].wix = index;
        self.workspaces[current].unmap_clients(&self.conn);
        self.workspaces[index].map_clients(&self.conn);
        self.apply_layout(self.focused_screen);
    }

    pub fn client_to_workspace(&mut self, index: usize) {
        debug!("moving focused client to workspace: {}", index);
        let ws = self.workspace_for_screen_mut(self.focused_screen);
        let client = match ws.remove_focused_client() {
            Some(client) => client,
            None => return,
        };

        self.workspaces[index].add_client(client);
        self.apply_layout(self.focused_screen);

        for (i, screen) in self.screens.iter().enumerate() {
            if screen.wix == index {
                self.apply_layout(i);
            }
        }
    }

    pub fn next_client(&mut self) {
        self.cycle_client(Direction::Forward);
    }

    pub fn previous_client(&mut self) {
        self.cycle_client(Direction::Backward);
    }

    pub fn kill_client(&mut self) {
        let id = match self.focused_client() {
            Some(client) => client.id,
            None => return,
        };
        let wm_delete_window = intern_atom(&self.conn, "WM_DELETE_WINDOW");
        let wm_protocols = intern_atom(&self.conn, "WM_PROTOCOLS");
        let data =
            xcb::ClientMessageData::from_data32([wm_delete_window, xcb::CURRENT_TIME, 0, 0, 0]);
        let event = xcb::ClientMessageEvent::new(32, id, wm_protocols, data);
        xcb::send_event(&self.conn, false, id, xcb::EVENT_MASK_NO_EVENT, &event);
        self.conn.flush();

        self.remove_client(id);
        self.next_client();
        self.apply_layout(self.focused_screen);
    }

    pub fn next_layout(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .cycle_layout(Direction::Forward);
        self.apply_layout(self.focused_screen);
    }

    pub fn previous_layout(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .cycle_layout(Direction::Backward);
        self.apply_layout(self.focused_screen);
    }

    pub fn inc_main(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .update_max_main(Change::More);
        self.apply_layout(self.focused_screen);
    }

    pub fn dec_main(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .update_max_main(Change::Less);
        self.apply_layout(self.focused_screen);
    }

    pub fn inc_ratio(&mut self) {
        let step = self.main_ratio_step;
        self.workspace_for_screen_mut(self.focused_screen)
            .update_main_ratio(Change::More, step);
        self.apply_layout(self.focused_screen);
    }

    pub fn dec_ratio(&mut self) {
        let step = self.main_ratio_step;
        self.workspace_for_screen_mut(self.focused_screen)
            .update_main_ratio(Change::Less, step);
        self.apply_layout(self.focused_screen);
    }
}
