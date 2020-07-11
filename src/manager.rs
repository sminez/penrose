//! Main logic for running Penrose
use crate::client::Client;
use crate::data_types::{
    Change, ColorScheme, Config, Direction, KeyBindings, KeyCode, Region, WinId,
};
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
    client_map: HashMap<WinId, Client>,
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
    show_bar: bool,
    // respect_resize_hints: bool,
}

impl<'a> WindowManager<'a> {
    /// Initialise a new window manager instance using an existing connection to
    /// the X server.
    pub fn init(conf: Config, conn: &'a dyn XConn) -> WindowManager {
        let mut screens = conn.current_outputs();
        log!("connected to X server: {} screens detected", screens.len());
        for (i, s) in screens.iter().enumerate() {
            log!("screen ({}) :: {:?}", i, s);
        }

        screens
            .iter_mut()
            .for_each(|s| s.update_effective_region(conf.bar_height, conf.top_bar));

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
            show_bar: conf.show_bar,
            // respect_resize_hints: conf.respect_resize_hints,
        }
    }

    fn apply_layout(&self, workspace: usize) {
        let ws = &self.workspaces[workspace];
        let s = self.screens.iter().find(|s| s.wix == workspace).unwrap();

        for (id, region) in ws.arrange(s.region(self.show_bar)) {
            debug!("configuring {} with {:?}", id, region);
            let (x, y, w, h) = region.values();
            let padding = 2 * (self.border_px + self.gap_px);
            let r = Region::new(x + self.gap_px, y + self.gap_px, w - padding, h - padding);
            self.conn.position_window(id, r, self.border_px);
        }
    }

    fn remove_client(&mut self, win_id: WinId) {
        match self.client_map.get(&win_id) {
            Some(client) => {
                self.workspaces[client.workspace()].remove_client(win_id);
                self.client_map.remove(&win_id).map(|c| {
                    debug!("removing ref to client {} ({})", c.id(), c.class());
                });
            }
            None => warn!("attempt to remove unknown client {}", win_id),
        }
    }

    /*
     * Helpers for indexing into WindowManager state
     */

    fn workspace_for_screen_mut(&mut self, screen_index: usize) -> &mut Workspace {
        &mut self.workspaces[self.screens[screen_index].wix]
    }

    fn workspace_for_client_mut(&mut self, id: WinId) -> Option<&mut Workspace> {
        if let Some(client) = self.client_map.get(&id) {
            Some(&mut self.workspaces[client.workspace()])
        } else {
            None
        }
    }

    fn active_ws_index(&self) -> usize {
        self.screens[self.focused_screen].wix
    }

    fn focused_client(&self) -> Option<&Client> {
        self.workspaces[self.active_ws_index()]
            .focused_client()
            .and_then(|id| self.client_map.get(id))
    }

    fn cycle_client(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        let cycled = self.workspaces[wix].cycle_client(direction);

        if let Some((prev, new)) = cycled {
            self.handle_leave_notify(prev); // treat like losing x focus
            self.handle_enter_notify(new); // treat like gaining x focus
        }
    }

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn grab_keys_and_run(&mut self, bindings: KeyBindings) {
        self.conn.grab_keys(&bindings);
        self.focus_workspace(0);

        loop {
            if let Some(event) = self.conn.wait_for_event() {
                match event {
                    XEvent::KeyPress { code } => self.handle_key_press(code, &bindings),
                    XEvent::Map { window, ignore } => self.handle_map_notify(window, ignore),
                    XEvent::Enter { window } => self.handle_enter_notify(window),
                    XEvent::Leave { window } => self.handle_leave_notify(window),
                    XEvent::Destroy { window } => self.handle_destroy_notify(window),
                    // XEvent::ButtonPress => self.handle_button_press(),
                    // XEvent::ButtonRelease => self.handle_button_release(),
                    _ => (),
                }
            }

            self.conn.flush();
        }
    }

    /*
     * X Event handler functions
     * These are called in response to incoming XEvents so calling them directly should
     * only be done if the intent is to act as if the corresponding XEvent had been
     * received from the X event loop (i.e. to avoid emitting and picking up the event
     * ourselves)
     */

    fn handle_key_press(&mut self, key_code: KeyCode, bindings: &KeyBindings) {
        if let Some(action) = bindings.get(&key_code) {
            debug!("handling key code: {:?}", key_code);
            action(self);
        }
    }

    pub fn handle_map_notify(&mut self, win_id: WinId, override_redirect: bool) {
        if override_redirect {
            return;
        } else if self.client_map.contains_key(&win_id) {
            warn!("got map request for known client: {}", win_id);
            return;
        }

        let wm_class = match self.conn.str_prop(win_id, "WM_CLASS") {
            Ok(s) => s.split("\0").collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        };

        debug!("handling new window: {}", wm_class);
        let floating = self.floating_classes.contains(&wm_class.as_ref());
        let wix = self.active_ws_index();
        let client = Client::new(win_id, wm_class, wix, floating);

        self.client_map.insert(win_id, client);
        if !floating {
            self.workspaces[wix].add_client(win_id);
        }

        self.conn.focus_client(win_id);
        self.conn.mark_new_window(win_id);
        let color = self.color_scheme.highlight;
        self.conn.set_client_border_color(win_id, color);
        self.apply_layout(self.active_ws_index());
    }

    fn handle_enter_notify(&mut self, id: WinId) {
        debug!("focusing client {}", id);
        let color_focus = self.color_scheme.highlight;
        let color_normal = self.color_scheme.fg_1;
        self.focused_client()
            .map(|c| self.conn.set_client_border_color(c.id(), color_normal));
        self.conn.focus_client(id);
        self.conn.set_client_border_color(id, color_focus);
        self.workspace_for_client_mut(id)
            .and_then(|ws| ws.focus_client(id));
    }

    fn handle_leave_notify(&self, id: WinId) {
        debug!("unfocusing client {}", id);
        let color = self.color_scheme.fg_1;
        self.conn.set_client_border_color(id, color);
        // TODO: do we need to explicitly cycle focus?
    }

    // fn handle_motion_notify(&mut self, event: &xcb::MotionNotifyEvent) {}
    // fn handle_button_press(&mut self, event: &xcb::ButtonPressEvent) {}
    // fn handle_button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}

    fn handle_destroy_notify(&mut self, win_id: WinId) {
        self.remove_client(win_id);
        self.apply_layout(self.active_ws_index());
    }

    /*
     * Public methods that can be triggered by user bindings
     *
     * User defined hooks can be implemented by adding additional logic to these
     * handlers which will then be run each time they are triggered
     */

    /// Shut down the WindowManager, running any required cleanup and exiting penrose
    pub fn exit(&mut self) {
        self.conn.flush();
        process::exit(0);
    }

    /**
     * Set the displayed workspace for the focused screen to be `index` in the list of
     * workspaces passed at `init`. This will panic if the index passed is out of
     * bounds which is only possible if you manually bind an action to this with an
     * invalid index. You should almost always be using the `gen_keybindings!` macro
     * to set up your keybindings so this is not normally an issue.
     */
    pub fn focus_workspace(&mut self, index: usize) {
        if self.active_ws_index() == index {
            return; // already focused on the current screen
        }

        for i in 0..self.screens.len() {
            if self.screens[i].wix == index {
                // The workspace we want is currently displayed on another screen so
                // pull the target workspace to the focused screen, and place the
                // workspace we had on the screen where the target was
                self.screens[i].wix = self.screens[self.focused_screen].wix;
                self.screens[self.focused_screen].wix = index;

                // re-apply layouts as screen dimensions may differ
                self.apply_layout(self.active_ws_index());
                self.apply_layout(index);
                return;
            }
        }

        // target not currently displayed so unmap what we currently have
        // displayed and replace it with the target workspace
        self.workspaces[self.active_ws_index()]
            .iter()
            .for_each(|c| self.conn.unmap_window(*c));

        self.workspaces[index]
            .iter()
            .for_each(|c| self.conn.map_window(*c));

        self.screens[self.focused_screen].wix = index;
        self.apply_layout(self.active_ws_index());
    }

    /**
     * Move the focused client to the workspace at `index` in the workspaces list.
     * This will panic if you pass an index that is out of bounds.
     */
    pub fn client_to_workspace(&mut self, index: usize) {
        if index == self.screens[self.focused_screen].wix {
            return;
        }

        debug!("moving focused client to workspace: {}", index);
        let ws = self.workspace_for_screen_mut(self.focused_screen);
        ws.remove_focused_client().map(|id| {
            self.conn.unmap_window(id);
            self.workspaces[index].add_client(id);
            self.client_map.get_mut(&id).map(|c| c.set_workspace(index));
            self.apply_layout(self.active_ws_index());
        });
    }

    /// Move focus to the next client in the stack
    pub fn next_client(&mut self) {
        self.cycle_client(Direction::Forward);
    }

    /// Move focus to the previous client in the stack
    pub fn previous_client(&mut self) {
        self.cycle_client(Direction::Backward);
    }

    /// Kill the focused client window.
    pub fn kill_client(&mut self) {
        if let Some(client) = self.focused_client() {
            let id = client.id();
            self.conn.send_client_event(id, "WM_DELETE_WINDOW");
            self.conn.flush();

            self.remove_client(id);
            self.apply_layout(self.active_ws_index());
        }
    }

    /// Rearrange the windows on the focused screen using the next available layout
    pub fn next_layout(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .cycle_layout(Direction::Forward);
        self.apply_layout(self.active_ws_index());
    }

    /// Rearrange the windows on the focused screen using the previous layout
    pub fn previous_layout(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .cycle_layout(Direction::Backward);
        self.apply_layout(self.active_ws_index());
    }

    /// Increase the number of windows in the main layout area
    pub fn inc_main(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .update_max_main(Change::More);
        self.apply_layout(self.active_ws_index());
    }

    /// Reduce the number of windows in the main layout area
    pub fn dec_main(&mut self) {
        self.workspace_for_screen_mut(self.focused_screen)
            .update_max_main(Change::Less);
        self.apply_layout(self.active_ws_index());
    }

    /// Make the main area larger relative to sub-areas
    pub fn inc_ratio(&mut self) {
        let step = self.main_ratio_step;
        self.workspace_for_screen_mut(self.focused_screen)
            .update_main_ratio(Change::More, step);
        self.apply_layout(self.active_ws_index());
    }

    /// Make the main area smaller relative to sub-areas
    pub fn dec_ratio(&mut self) {
        let step = self.main_ratio_step;
        self.workspace_for_screen_mut(self.focused_screen)
            .update_main_ratio(Change::Less, step);
        self.apply_layout(self.active_ws_index());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::*;
    use crate::layout::*;
    use crate::screen::*;
    use crate::xconnection::*;

    const FONTS: &[&str] = &["Comic Sans:size=88"];
    const WORKSPACES: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
    const FLOATING_CLASSES: &[&str] = &["clouds", "birds"];
    const COLOR_SCHEME: ColorScheme = ColorScheme {
        bg: 0x282828,        // #282828
        fg_1: 0x3c3836,      // #3c3836
        fg_2: 0xa89984,      // #a89984
        fg_3: 0xf2e5bc,      // #f2e5bc
        highlight: 0xcc241d, // #cc241d
        urgent: 0x458588,    // #458588
    };

    fn wm_with_mock_conn<'a>(layouts: Vec<Layout>, conn: &'a MockXConn) -> WindowManager<'a> {
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
            bar_height: 18,
            respect_resize_hints: true,
        };

        WindowManager::init(conf, conn)
    }

    fn test_layouts() -> Vec<Layout> {
        vec![Layout::new("t", LayoutKind::Normal, mock_layout, 1, 0.6)]
    }

    fn test_screens() -> Vec<Screen> {
        let r = Region::new(0, 0, 1366, 768);
        vec![Screen {
            true_region: r,
            effective_region: r,
            wix: 0,
        }]
    }

    fn add_n_clients(wm: &mut WindowManager, n: usize, offset: usize) {
        for i in 0..n {
            wm.handle_map_notify(10 * (i + offset + 1) as u32, false);
        }
    }

    #[test]
    fn worspace_switching_with_active_clients() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);

        // add clients to the first workspace: final client should have focus
        add_n_clients(&mut wm, 3, 0);
        assert_eq!(wm.workspaces[0].len(), 3);
        assert_eq!(*wm.workspaces[0].focused_client().unwrap(), 30);

        // switch and add to the second workspace: final client should have focus
        wm.focus_workspace(1);
        add_n_clients(&mut wm, 2, 3);
        assert_eq!(wm.workspaces[1].len(), 2);
        assert_eq!(*wm.workspaces[1].focused_client().unwrap(), 50);

        // switch back: clients should be the same, same client should have focus
        wm.focus_workspace(0);
        assert_eq!(wm.workspaces[0].len(), 3);
        assert_eq!(*wm.workspaces[0].focused_client().unwrap(), 30);
    }

    #[test]
    fn killing_a_client_removes_it_from_the_workspace() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0);
        wm.kill_client();

        assert_eq!(wm.workspaces[0].len(), 0);
    }

    #[test]
    fn kill_client_kills_focused_not_first() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 5, 0); // 50 40 30 20 10, 50 focused
        wm.next_client(); // 40 focused
        wm.kill_client(); // remove 40, focus 30

        let ids: Vec<WinId> = wm.workspaces[0].iter().map(|c| *c).collect();
        assert_eq!(ids, vec![50, 30, 20, 10]);
        assert_eq!(*wm.workspaces[0].focused_client().unwrap(), 30);
    }

    #[test]
    fn moving_then_deleting_clients() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0);
        wm.client_to_workspace(1);
        wm.client_to_workspace(1);
        wm.focus_workspace(1);
        wm.kill_client();

        // should have removed first client on ws::1 (last sent from ws::0)
        assert_eq!(wm.workspaces[1].iter().collect::<Vec<&WinId>>(), vec![&20]);
    }

    #[test]
    fn sending_a_client_inserts_at_head() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(1); // 20 -> ws::1
        wm.client_to_workspace(1); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(1);

        assert_eq!(
            wm.workspaces[1].iter().collect::<Vec<&WinId>>(),
            vec![&10, &20]
        );
    }

    #[test]
    fn sending_a_client_sets_focus() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(1); // 20 -> ws::1
        wm.client_to_workspace(1); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(1);

        assert_eq!(*wm.workspaces[1].focused_client().unwrap(), 10);
    }

    #[test]
    fn x_focus_events_set_workspace_focus() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 5, 0); // focus on last client: 50
        wm.handle_enter_notify(10);

        assert_eq!(*wm.workspaces[0].focused_client().unwrap(), 10);
    }
}
