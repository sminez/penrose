use crate::client::Client;
use crate::data_types::{
    Change, ColorScheme, Config, Direction, KeyBindings, KeyCode, Region, WinId,
};
use crate::helpers::spawn;
use crate::screen::Screen;
use crate::workspace::Workspace;
use crate::xconnection::{XConn, XEvent};
use std::collections::HashMap;
use std::process;

/**
 * WindowManager is the primary struct / owner of the event loop ofr penrose.
 * It handles most (if not all) of the communication with XCB and responds to
 * X events served over the embedded connection. User input bindings are parsed
 * and bound on init and then triggered via grabbed X events in the main loop
 * along with everything else.
 */
pub struct WindowManager<'a> {
    conn: &'a dyn XConn,
    screens: Vec<Screen>,
    workspaces: Vec<Workspace>,
    client_map: HashMap<WinId, usize>,
    focused_screen: usize,
    // config
    // fonts: &'static [&'static str],
    floating_classes: &'static [&'static str],
    color_scheme: ColorScheme,
    border_px: u32,
    gap_px: u32,
    main_ratio_step: f32,
    // systray_spacing_px: u32,
    // show_systray: bool,
    // show_bar: bool,
    // top_bar: bool,
    // respect_resize_hints: bool,
}

impl<'a> WindowManager<'a> {
    pub fn init(conf: Config, conn: &'a dyn XConn) -> WindowManager {
        let screens = conn.current_outputs();
        log!("connected to X server: {} screens detected", screens.len());

        let workspaces: Vec<Workspace> = conf
            .workspaces
            .iter()
            .map(|name| Workspace::new(name, conf.layouts.clone().to_vec()))
            .collect();

        WindowManager {
            conn: conn,
            screens,
            workspaces,
            client_map: HashMap::new(),
            focused_screen: 0,
            // fonts: conf.fonts,
            floating_classes: conf.floating_classes,
            color_scheme: conf.color_scheme,
            border_px: conf.border_px,
            gap_px: conf.gap_px,
            main_ratio_step: conf.main_ratio_step,
            // systray_spacing_px: conf.systray_spacing_px,
            // show_systray: conf.show_systray,
            // show_bar: conf.show_bar,
            // top_bar: conf.top_bar,
            // respect_resize_hints: conf.respect_resize_hints,
        }
    }

    fn apply_layout(&self, screen: usize) {
        let screen_region = self.screens[screen].region;
        let ws = self.workspace_for_screen(screen);

        for (id, region) in ws.arrange(&screen_region) {
            debug!("configuring {} with {:?}", id, region);
            let (x, y, w, h) = region.values();
            let padding = 2 * (self.border_px + self.gap_px);
            let r = Region::new(x + self.gap_px, y + self.gap_px, w - padding, h - padding);
            self.conn.position_window(id, r, self.border_px);
        }
    }

    fn remove_client(&mut self, win_id: WinId) {
        match self.client_map.get(&win_id) {
            Some(ix) => {
                debug!("removing ref to client {}", win_id);
                self.workspaces[*ix].remove_client(win_id);
                self.client_map.remove(&win_id);
            }
            None => warn!("attempt to remove unknown window {}", win_id),
        }
    }

    // fn button_press(&mut self, event: &xcb::ButtonPressEvent) {}

    // fn button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}

    fn key_press(&mut self, key_code: KeyCode, bindings: &KeyBindings) {
        if let Some(action) = bindings.get(&key_code) {
            debug!("handling key code: {:?}", key_code);
            action(self);
        } else {
            warn!("caught unknown key code: {:?}", key_code);
        }
    }

    fn map_x_window(&mut self, win_id: WinId) {
        if self.client_map.contains_key(&win_id) {
            return;
        }

        let wm_class = match self.conn.str_prop(win_id, "WM_CLASS") {
            Ok(s) => s.split("\0").collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        };

        debug!("handling new window: {}", wm_class);
        let floating = self.floating_classes.contains(&wm_class.as_ref());
        let client = Client::new(win_id, wm_class, floating);
        let wix = self.screens[self.focused_screen].wix;

        self.client_map.insert(win_id, wix);
        self.workspaces[wix].add_client(client);
        self.conn.focus_client(win_id);
        self.conn.mark_new_window(win_id);
        self.conn
            .set_client_border_color(win_id, self.color_scheme.highlight);

        self.apply_layout(self.focused_screen);
    }

    fn focus_window(&mut self, win_id: WinId) {
        debug!("focusing client {}", win_id);
        for ws in self.workspaces.iter_mut() {
            ws.focus_client(win_id, self.conn, &self.color_scheme);
        }
    }

    fn unfocus_window(&mut self, win_id: WinId) {
        for ws in self.workspaces.iter_mut() {
            match ws.focused_client_mut() {
                Some(client) => {
                    if client.id() == win_id {
                        client.unfocus(self.conn, &self.color_scheme);
                        return;
                    }
                }
                None => (),
            }
        }
    }

    // fn resize_window(&mut self, event: &xcb::MotionNotifyEvent) {}

    fn destroy_window(&mut self, win_id: WinId) {
        self.remove_client(win_id);
        self.apply_layout(self.focused_screen);
    }

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn grab_keys_and_run(&mut self, bindings: KeyBindings) {
        self.conn.grab_keys(&bindings);
        self.switch_workspace(0);

        loop {
            if let Some(event) = self.conn.wait_for_event() {
                match event {
                    XEvent::KeyPress(key_code) => self.key_press(key_code, &bindings),
                    XEvent::Map(win_id) => self.map_x_window(win_id),
                    XEvent::Enter(win_id) => self.focus_window(win_id),
                    XEvent::Leave(win_id) => self.unfocus_window(win_id),
                    XEvent::Destroy(win_id) => self.destroy_window(win_id),
                    // XEvent::Motion => self.resize_window(),
                    // XEvent::ButtonPress => self.button_press(),
                    // XEvent::ButtonRelease => self.button_release(),
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
            .cycle_client(direction, self.conn, &scheme);
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
            0 => spawn("xsetroot -solid #282828"),
            1 => spawn("xsetroot -solid #cc241d"),
            2 => spawn("xsetroot -solid #458588"),
            3 => spawn("xsetroot -solid #fabd2f"),
            4 => spawn("xsetroot -solid #b8bb26"),
            _ => spawn("xsetroot -solid #ebdbb2"),
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
        self.workspaces[current].unmap_clients(self.conn);
        self.workspaces[index].map_clients(self.conn);
        self.apply_layout(self.focused_screen);
    }

    pub fn client_to_workspace(&mut self, index: usize) {
        debug!("moving focused client to workspace: {}", index);
        let ws = self.workspace_for_screen_mut(self.focused_screen);
        let client = match ws.remove_focused_client() {
            Some(client) => client,
            None => return,
        };

        self.client_map.insert(client.id(), index);
        self.conn.unmap_window(client.id());
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
            Some(client) => client.id(),
            None => return,
        };

        self.conn.send_client_event(id, "WM_DELETE_WINDOW");
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

#[cfg(test)]
mod tests {
    /*
     * NOTE: using mockiato for mocking
     *       https://docs.rs/mockiato/0.9.5/mockiato/
     */

    use super::*;
    use crate::data_types::*;
    use crate::layout::*;
    use crate::screen::*;
    use crate::xconnection::*;

    const FONTS: &[&str] = &["ProFont For Powerline:size=10", "Iosevka Nerd Font:size=10"];
    const WORKSPACES: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
    const FLOATING_CLASSES: &[&str] = &["rofi", "dmenu", "dunst"];
    const COLOR_SCHEME: ColorScheme = ColorScheme {
        bg: 0x282828,        // #282828
        fg_1: 0x3c3836,      // #3c3836
        fg_2: 0xa89984,      // #a89984
        fg_3: 0xf2e5bc,      // #f2e5bc
        highlight: 0xcc241d, // #cc241d
        urgent: 0x458588,    // #458588
    };

    fn mock_conn() -> impl XConn {
        let mut conn = XConnMock::new();
        conn.expect_current_outputs().times(1).returns(vec![Screen {
            region: Region::new(0, 0, 1366, 768),
            wix: 0,
        }]);

        conn
    }

    fn wm_with_mock_conn<'a>(layouts: Vec<Layout>, conn: &'a dyn XConn) -> WindowManager<'a> {
        let conf = Config {
            workspaces: WORKSPACES,
            fonts: FONTS,
            floating_classes: FLOATING_CLASSES,
            layouts: layouts,
            color_scheme: COLOR_SCHEME,
            border_px: 2,
            gap_px: 5,
            main_ratio_step: 0.05,
            systray_spacing_px: 2,
            show_systray: true,
            show_bar: true,
            top_bar: true,
            respect_resize_hints: true,
        };

        WindowManager::init(conf, conn)
    }
}
