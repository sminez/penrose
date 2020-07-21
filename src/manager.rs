//! Main logic for running Penrose
use crate::client::Client;
use crate::data_types::{
    Change, ColorScheme, Config, Direction, KeyBindings, KeyCode, Point, Region, Ring, WinId,
};
use crate::screen::Screen;
use crate::workspace::Workspace;
use crate::xconnection::{XConn, XEvent};
use std::collections::HashMap;
use std::process::{exit, Child};

/**
 * WindowManager is the primary struct / owner of the event loop ofr penrose.
 * It handles most (if not all) of the communication with XCB and responds to
 * X events served over the embedded connection. User input bindings are parsed
 * and bound on init and then triggered via grabbed X events in the main loop
 * along with everything else.
 */
pub struct WindowManager<'a> {
    conn: &'a dyn XConn,
    screens: Ring<Screen>,
    workspaces: Ring<Workspace>,
    client_map: HashMap<WinId, Client>,
    previous_workspace: usize,
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
        info!("connected to X server: {} screens detected", screens.len());
        for (i, s) in screens.iter().enumerate() {
            info!("screen ({}) :: {:?}", i, s);
        }

        screens
            .iter_mut()
            .for_each(|s| s.update_effective_region(conf.bar_height, conf.top_bar));

        let workspaces: Vec<Workspace> = conf
            .workspaces
            .iter()
            .map(|name| Workspace::new(name, conf.layouts.clone().to_vec()))
            .collect();

        conn.set_wm_properties(conf.workspaces);

        WindowManager {
            conn: conn,
            screens: Ring::new(screens),
            workspaces: Ring::new(workspaces),
            client_map: HashMap::new(),
            previous_workspace: 0,
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
        let lc = ws.layout_conf();
        if lc.floating {
            return;
        }

        let s = self.screens.iter().find(|s| s.wix == workspace).unwrap();
        let gpx = if lc.gapless { 0 } else { self.gap_px };
        let padding = 2 * (self.border_px + gpx);

        for (id, region) in ws.arrange(s.region(self.show_bar), &self.client_map) {
            debug!("configuring {} with {:?}", id, region);
            let (x, y, w, h) = region.values();
            let r = Region::new(x + gpx, y + gpx, w - padding, h - padding);
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

    fn set_screen_from_cursor(&mut self, cursor: Point) -> Option<&Screen> {
        let s = self.screens.focus_by(|s| s.contains(cursor));
        debug!("FOCUSED_SCREEN :: {:?}", s);
        s
    }

    fn workspace_index_for_client(&mut self, id: WinId) -> Option<usize> {
        self.client_map.get(&id).map(|c| c.workspace())
    }

    fn active_ws_index(&self) -> usize {
        self.screens.focused().unwrap().wix
    }

    fn focused_client(&self) -> Option<&Client> {
        self.workspaces[self.active_ws_index()]
            .focused_client()
            .and_then(|id| self.client_map.get(&id))
    }

    fn client_gained_focus(&mut self, id: WinId) {
        let color_focus = self.color_scheme.highlight;
        let color_normal = self.color_scheme.fg_1;
        self.focused_client()
            .map(|c| self.conn.set_client_border_color(c.id(), color_normal));
        self.conn.focus_client(id);
        self.conn.set_client_border_color(id, color_focus);

        if let Some(wix) = self.workspace_index_for_client(id) {
            let ws = &mut self.workspaces[wix];
            ws.focus_client(id);
            if ws.layout_conf().follow_focus {
                self.apply_layout(wix);
            }
        }
    }

    fn client_lost_focus(&mut self, id: WinId) {
        let color = self.color_scheme.fg_1;
        self.conn.set_client_border_color(id, color);
    }

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn grab_keys_and_run(&mut self, bindings: KeyBindings) {
        // TODO: need to be smarter about this. This will also map all of the systray apps
        //       as tiled windows currently.
        // for id in self.conn.query_for_active_windows() {
        //     self.handle_map_notify(id, false);
        // }

        self.conn.grab_keys(&bindings);
        self.focus_workspace(0);

        let mut spawned = Vec::new();

        loop {
            if let Some(event) = self.conn.wait_for_event() {
                match event {
                    XEvent::KeyPress { code } => {
                        self.handle_key_press(code, &bindings, &mut spawned)
                    }
                    XEvent::Map { id, ignore } => self.handle_map_notify(id, ignore),
                    XEvent::Enter { id, rpt, wpt } => self.handle_enter_notify(id, rpt, wpt),
                    XEvent::Leave { id, rpt, wpt } => self.handle_leave_notify(id, rpt, wpt),
                    XEvent::Destroy { id } => self.handle_destroy_notify(id),
                    XEvent::ScreenChange => self.handle_screen_change(),
                    // XEvent::ButtonPress => self.handle_button_press(),
                    // XEvent::ButtonRelease => self.handle_button_release(),
                    _ => (),
                }
            }

            self.conn.flush();

            // reap any spawned child processes that have now completed
            spawned = spawned
                .into_iter()
                .filter_map(|mut c| {
                    match c.try_wait() {
                        Ok(None) => Some(c), // still running
                        Ok(Some(_)) => None, // clean exit
                        Err(e) => {
                            warn!("subprocess [{}] errored: {}", c.id(), e);
                            None
                        }
                    }
                })
                .collect();
        }
    }

    /*
     * X Event handler functions
     * These are called in response to incoming XEvents so calling them directly should
     * only be done if the intent is to act as if the corresponding XEvent had been
     * received from the X event loop (i.e. to avoid emitting and picking up the event
     * ourselves)
     */

    fn handle_key_press(
        &mut self,
        key_code: KeyCode,
        bindings: &KeyBindings,
        spawned: &mut Vec<Child>,
    ) {
        if let Some(action) = bindings.get(&key_code) {
            debug!("handling key code: {:?}", key_code);
            if let Some(child) = action(self) {
                spawned.push(child);
            }
        }
    }

    fn handle_map_notify(&mut self, win_id: WinId, override_redirect: bool) {
        if override_redirect || self.client_map.contains_key(&win_id) {
            return;
        }

        let wm_class = match self.conn.str_prop(win_id, "WM_CLASS") {
            Ok(s) => s.split("\0").collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        };

        let floating = self.floating_classes.contains(&wm_class.as_ref());
        let wix = self.active_ws_index();
        let client = Client::new(win_id, wm_class, wix, floating);
        debug!("mapping client: {:?}", client);

        self.client_map.insert(win_id, client);
        if !floating {
            self.workspaces[wix].add_client(win_id);
        }

        self.conn.focus_client(win_id);
        self.conn.mark_new_window(win_id);
        let color = self.color_scheme.highlight;
        self.conn.set_client_border_color(win_id, color);
        self.conn.set_client_workspace(win_id, wix);
        self.apply_layout(self.active_ws_index());
    }

    fn handle_enter_notify(&mut self, id: WinId, rpt: Point, _wpt: Point) {
        self.client_gained_focus(id);
        self.set_screen_from_cursor(rpt);
    }

    fn handle_leave_notify(&mut self, id: WinId, rpt: Point, _wpt: Point) {
        self.client_lost_focus(id);
        self.set_screen_from_cursor(rpt);
    }

    fn handle_screen_change(&mut self) {
        self.set_screen_from_cursor(self.conn.cursor_position());
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

    /// Cycle between known screens. Does not wrap from first to last
    pub fn cycle_screen(&mut self, direction: Direction) {
        if !self.screens.would_wrap(direction) {
            self.screens.cycle_focus(direction);
            self.workspaces
                .focus_nth(self.screens.focused().unwrap().wix);
            self.conn.warp_cursor(None, self.screens.focused().unwrap());
        }
    }

    /**
     * Cycle between workspaces on the current Screen. This will pull workspaces
     * to the screen if they are currently displayed on another screen.
     */
    pub fn cycle_workspace(&mut self, direction: Direction) {
        self.workspaces.cycle_focus(direction);
        let i = self.workspaces.focused_index();
        self.focus_workspace(i);
    }

    pub fn drag_workspace(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        self.cycle_screen(direction);
        self.focus_workspace(wix); // focus_workspace will pull it to the new screen
    }

    /// Cycle between Clients for the active Workspace
    pub fn cycle_client(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        let cycled = self.workspaces[wix].cycle_client(direction);

        if let Some((prev, new)) = cycled {
            self.client_lost_focus(prev);
            self.client_gained_focus(new);
            self.conn
                .warp_cursor(Some(new), self.screens.focused().unwrap());
        }
    }

    /// Move the focused Client through the stack of Clients on the active Workspace
    pub fn drag_client(&mut self, direction: Direction) {
        if let Some(id) = self.focused_client().and_then(|c| Some(c.id())) {
            let wix = self.active_ws_index();
            self.workspaces[wix].drag_client(direction);
            self.apply_layout(wix);
            self.client_gained_focus(id);
            self.conn
                .warp_cursor(Some(id), self.screens.focused().unwrap());
        }
    }

    /// Cycle between Layouts for the active Workspace
    pub fn cycle_layout(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        self.workspaces[wix].cycle_layout(direction);
        self.apply_layout(wix);
        info!("ACTIVE_LAYOUT {}", self.workspaces[wix].layout_symbol());
    }

    /// Increase or decrease the number of clients in the main area by 1
    pub fn update_max_main(&mut self, change: Change) {
        let wix = self.active_ws_index();
        self.workspaces[wix].update_max_main(change);
        self.apply_layout(wix);
    }

    /// Increase or decrease the current Layout main_ratio by main_ratio_step
    pub fn update_main_ratio(&mut self, change: Change) {
        let step = self.main_ratio_step;
        let wix = self.active_ws_index();
        self.workspaces[wix].update_main_ratio(change, step);
        self.apply_layout(wix);
    }

    /// Shut down the WindowManager, running any required cleanup and exiting penrose
    pub fn exit(&mut self) {
        self.conn.cleanup();
        self.conn.flush();
        exit(0);
    }

    /// The layout symbol for the Layout currently being used on the active workspace
    pub fn current_layout_symbol(&self) -> &str {
        self.workspaces[self.active_ws_index()].layout_symbol()
    }

    /// Set the root X window name. Useful for exposing information to external programs
    pub fn set_root_window_name(&self, s: &str) {
        self.conn.set_root_window_name(s);
    }

    /**
     * Set the displayed workspace for the focused screen to be `index` in the list of
     * workspaces passed at `init`. This will panic if the index passed is out of
     * bounds which is only possible if you manually bind an action to this with an
     * invalid index. You should almost always be using the `gen_keybindings!` macro
     * to set up your keybindings so this is not normally an issue.
     */
    pub fn focus_workspace(&mut self, index: usize) {
        info!("ACTIVE_LAYOUT {}", self.workspaces[index].layout_symbol());
        let active = self.active_ws_index();

        if active == index {
            return; // already focused on the current screen
        } else {
            self.previous_workspace = active
        }

        for i in 0..self.screens.len() {
            if self.screens[i].wix == index {
                // The workspace we want is currently displayed on another screen so
                // pull the target workspace to the focused screen, and place the
                // workspace we had on the screen where the target was
                self.screens[i].wix = self.screens.focused().unwrap().wix;
                self.screens.focused_mut().unwrap().wix = index;

                // re-apply layouts as screen dimensions may differ
                self.apply_layout(active);
                self.apply_layout(index);
                return;
            }
        }

        // target not currently displayed so unmap what we currently have
        // displayed and replace it with the target workspace
        self.workspaces[active]
            .iter()
            .for_each(|c| self.conn.unmap_window(*c));

        self.workspaces[index]
            .iter()
            .for_each(|c| self.conn.map_window(*c));

        self.screens.focused_mut().unwrap().wix = index;
        self.apply_layout(index);
        self.conn.set_current_workspace(index);
    }

    /// Switch focus back to the last workspace that had focus.
    pub fn toggle_workspace(&mut self) {
        self.focus_workspace(self.previous_workspace);
    }

    /**
     * Move the focused client to the workspace at `index` in the workspaces list.
     * This will panic if you pass an index that is out of bounds.
     */
    pub fn client_to_workspace(&mut self, index: usize) {
        if index == self.screens.focused().unwrap().wix {
            return;
        }

        let wix = self.active_ws_index();
        let ws = &mut self.workspaces[wix];
        ws.remove_focused_client().map(|id| {
            self.conn.unmap_window(id);
            self.workspaces[index].add_client(id);
            self.client_map.get_mut(&id).map(|c| c.set_workspace(index));
            self.conn.set_client_workspace(id, index);
            self.apply_layout(self.active_ws_index());
        });
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::{Direction::*, *};
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
        vec![Layout::new("t", LayoutConf::default(), mock_layout, 1, 0.6)]
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
        assert_eq!(wm.workspaces[0].focused_client(), Some(30));

        // switch and add to the second workspace: final client should have focus
        wm.focus_workspace(1);
        add_n_clients(&mut wm, 2, 3);
        assert_eq!(wm.workspaces[1].len(), 2);
        assert_eq!(wm.workspaces[1].focused_client(), Some(50));

        // switch back: clients should be the same, same client should have focus
        wm.focus_workspace(0);
        assert_eq!(wm.workspaces[0].len(), 3);
        assert_eq!(wm.workspaces[0].focused_client(), Some(30));
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
        assert_eq!(wm.active_ws_index(), 0);
        wm.cycle_client(Forward); // 40 focused
        assert_eq!(wm.workspaces[0].focused_client(), Some(40));
        wm.kill_client(); // remove 40, focus 30

        let ids: Vec<WinId> = wm.workspaces[0].iter().map(|c| *c).collect();
        assert_eq!(ids, vec![50, 30, 20, 10]);
        assert_eq!(wm.workspaces[0].focused_client(), Some(30));
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

        assert_eq!(wm.workspaces[1].focused_client(), Some(10));
    }

    #[test]
    fn x_focus_events_set_workspace_focus() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 5, 0); // focus on last client: 50
        wm.client_gained_focus(10);

        assert_eq!(wm.workspaces[0].focused_client(), Some(10));
    }

    #[test]
    fn dragging_clients_forward_from_index_0() {
        let conn = MockXConn::new(test_screens());
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 5, 0); // focus on last client (50) ix == 0

        let clients = |w: &mut WindowManager| {
            w.workspaces[w.screens[0].wix]
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };

        wm.drag_client(Forward);
        assert_eq!(wm.focused_client().unwrap().id(), 50);
        assert_eq!(clients(&mut wm), vec![40, 50, 30, 20, 10]);

        wm.drag_client(Forward);
        assert_eq!(wm.focused_client().unwrap().id(), 50);
        assert_eq!(clients(&mut wm), vec![40, 30, 50, 20, 10]);

        wm.client_gained_focus(20);
        wm.drag_client(Forward);
        assert_eq!(wm.focused_client().unwrap().id(), 20);
        assert_eq!(clients(&mut wm), vec![40, 30, 50, 10, 20]);
    }
}
