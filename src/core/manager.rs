//! Main logic for running Penrose
use crate::client::Client;
use crate::data_types::{
    Change, ColorScheme, Config, Direction, KeyBindings, KeyCode, Point, Region, Ring, Selector,
    WinId,
};
use crate::hooks;
use crate::screen::Screen;
use crate::workspace::Workspace;
use crate::xconnection::{XConn, XEvent};
use nix::sys::signal::{signal, SigHandler, Signal};
use std::cell::Cell;
use std::collections::HashMap;

type MutableHooks<T> = Cell<Vec<Box<T>>>;

// Relies on all hooks taking &mut WindowManager as the first arg.
macro_rules! run_hooks(
    ($hookvec:ident, $_self:expr, $($arg:expr),+) => {
        let mut hooks = $_self.$hookvec.replace(vec![]);
        hooks.iter_mut().for_each(|h| h.call($_self, $($arg),+));
        $_self.$hookvec.replace(hooks);
    };
);

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
    // fonts: &'static [&'static str],
    floating_classes: &'static [&'static str],
    color_scheme: ColorScheme,
    border_px: u32,
    gap_px: u32,
    main_ratio_step: f32,
    // systray_spacing_px: u32,
    // show_systray: bool,
    show_bar: bool,
    bar_height: u32,
    top_bar: bool,
    // respect_resize_hints: bool,
    new_client_hooks: MutableHooks<dyn hooks::NewClientHook>,
    layout_change_hooks: MutableHooks<dyn hooks::LayoutChangeHook>,
    workspace_change_hooks: MutableHooks<dyn hooks::WorkspaceChangeHook>,
    screen_change_hooks: MutableHooks<dyn hooks::ScreenChangeHook>,
    focus_change_hooks: MutableHooks<dyn hooks::FocusChangeHook>,
    running: bool,
}

impl<'a> WindowManager<'a> {
    /// Initialise a new window manager instance using an existing connection to the X server.
    pub fn init(config: Config, conn: &'a dyn XConn) -> WindowManager<'a> {
        let layouts = config.layouts.clone();
        let mut wm = WindowManager {
            conn: conn,
            screens: Ring::new(vec![]),
            workspaces: Ring::new(vec![]),
            client_map: HashMap::new(),
            previous_workspace: 0,
            // fonts: conf.fonts,
            floating_classes: config.floating_classes,
            color_scheme: config.color_scheme,
            border_px: config.border_px,
            gap_px: config.gap_px,
            main_ratio_step: config.main_ratio_step,
            // systray_spacing_px: conf.systray_spacing_px,
            // show_systray: conf.show_systray,
            show_bar: config.show_bar,
            bar_height: config.bar_height,
            top_bar: config.top_bar,
            // respect_resize_hints: conf.respect_resize_hints,
            new_client_hooks: Cell::new(config.new_client_hooks),
            layout_change_hooks: Cell::new(config.layout_change_hooks),
            workspace_change_hooks: Cell::new(config.workspace_change_hooks),
            screen_change_hooks: Cell::new(config.screen_change_hooks),
            focus_change_hooks: Cell::new(config.focus_change_hooks),
            running: false,
        };

        wm.workspaces = Ring::new(
            config
                .workspaces
                .iter()
                .map(|name| Workspace::new(name, layouts.to_vec()))
                .collect(),
        );
        wm.detect_screens();
        conn.set_wm_properties(config.workspaces);

        return wm;
    }

    fn apply_layout(&mut self, wix: usize) {
        let lc = self.workspaces[wix].layout_conf();
        if lc.floating {
            return;
        }

        let (i, s) = {
            self.screens
                .iter()
                .enumerate()
                .find(|(_, s)| s.wix == wix)
                .unwrap()
        };

        let r = s.region(self.show_bar);

        run_hooks!(layout_change_hooks, self, wix, i);

        let ws = &self.workspaces[wix];
        let gpx = if lc.gapless { 0 } else { self.gap_px };
        let padding = 2 * (self.border_px + gpx);

        for (id, region) in ws.arrange(r, &self.client_map) {
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
        self.screens
            .focus(Selector::Condition(&|s| s.contains(cursor)))
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

    fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.workspaces[self.active_ws_index()]
            .focused_client()
            .and_then(move |id| self.client_map.get_mut(&id))
    }

    fn client_gained_focus(&mut self, id: WinId) {
        run_hooks!(focus_change_hooks, self, id);

        self.focused_client()
            .map(|c| self.client_lost_focus(c.id()));

        let color = self.color_scheme.highlight;
        self.conn.set_client_border_color(id, color);
        self.conn.focus_client(id);

        if let Some(wix) = self.workspace_index_for_client(id) {
            let ws = &mut self.workspaces[wix];
            ws.focus_client(id);
            if ws.layout_conf().follow_focus {
                self.apply_layout(wix);
            }
        }
    }

    fn client_lost_focus(&self, id: WinId) {
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

        // ignore SIGCHILD and allow child / inherited processes to be inherited by pid1
        unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) }.unwrap();

        self.conn.grab_keys(&bindings);
        self.focus_workspace(0);
        self.running = true;

        while self.running {
            if let Some(event) = self.conn.wait_for_event() {
                debug!("got XEvent: {:?}", event);
                match event {
                    XEvent::KeyPress { code } => self.handle_key_press(code, &bindings),
                    XEvent::Map { id, ignore } => self.handle_map_notify(id, ignore),
                    XEvent::Enter { id, rpt, wpt } => self.handle_enter_notify(id, rpt, wpt),
                    XEvent::Leave { id, rpt, wpt } => self.handle_leave_notify(id, rpt, wpt),
                    XEvent::Destroy { id } => self.handle_destroy_notify(id),
                    XEvent::ScreenChange => self.handle_screen_change(),
                    XEvent::RandrNotify => self.detect_screens(),
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
            action(self); // ignoring Child handlers and SIGCHILD
        }
    }

    fn handle_map_notify(&mut self, id: WinId, override_redirect: bool) {
        if override_redirect || self.client_map.contains_key(&id) {
            return;
        }

        let wm_class = match self.conn.str_prop(id, "WM_CLASS") {
            Ok(s) => s.split("\0").collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        };

        let wm_name = match self.conn.str_prop(id, "WM_NAME") {
            Ok(s) => s,
            Err(_) => String::from("n/a"),
        };

        let floating = self.floating_classes.contains(&wm_class.as_ref());
        let wix = self.active_ws_index();
        let mut client = Client::new(id, wm_name, wm_class, wix, floating);
        debug!("mapping client: {:?}", client);
        run_hooks!(new_client_hooks, self, &mut client);

        self.client_map.insert(id, client);
        if !floating {
            self.workspaces[wix].add_client(id);
        }

        self.conn.mark_new_window(id);
        self.conn.focus_client(id);
        self.client_gained_focus(id);

        let s = self.screens.focused().unwrap();
        self.conn.warp_cursor(Some(id), s);

        self.conn.set_client_workspace(id, wix);
        self.apply_layout(self.active_ws_index());
    }

    fn handle_enter_notify(&mut self, id: WinId, rpt: Point, _wpt: Point) {
        if let Some(current) = self.focused_client() {
            if current.id() != id {
                self.client_lost_focus(current.id());
            }
        }

        self.client_gained_focus(id);
        self.set_screen_from_cursor(rpt);
    }

    fn handle_leave_notify(&mut self, id: WinId, rpt: Point, _wpt: Point) {
        self.client_lost_focus(id);
        self.set_screen_from_cursor(rpt);
    }

    fn handle_screen_change(&mut self) {
        self.set_screen_from_cursor(self.conn.cursor_position());
        let wix = self.screens.focused().unwrap().wix;
        self.workspaces.focus(Selector::Index(wix));
    }

    // fn handle_motion_notify(&mut self, event: &xcb::MotionNotifyEvent) {}
    // fn handle_button_press(&mut self, event: &xcb::ButtonPressEvent) {}
    // fn handle_button_release(&mut self, event: &xcb::ButtonReleaseEvent) {}

    fn handle_destroy_notify(&mut self, win_id: WinId) {
        debug!("DESTROY_NOTIFY for {}", win_id);
        self.remove_client(win_id);
        self.apply_layout(self.active_ws_index());
    }

    /*
     * Public methods that can be triggered by user bindings
     *
     * User defined hooks can be implemented by adding additional logic to these
     * handlers which will then be run each time they are triggered
     */

    /// Reset the current known screens based on currently detected outputs
    pub fn detect_screens(&mut self) {
        let screens: Vec<Screen> = self
            .conn
            .current_outputs()
            .into_iter()
            .enumerate()
            .map(|(i, mut s)| {
                s.update_effective_region(self.bar_height, self.top_bar);
                s.wix = i;
                s
            })
            .collect();

        info!("updating known screens: {} screens detected", screens.len());
        for (i, s) in screens.iter().enumerate() {
            info!("screen ({}) :: {:?}", i, s);
        }

        if screens == self.screens.as_vec() {
            return;
        }

        self.screens = Ring::new(screens);
        let visible_workspaces: Vec<_> = self.screens.iter().map(|s| s.wix).collect();
        visible_workspaces
            .iter()
            .for_each(|wix| self.apply_layout(*wix));
    }

    /// Log information out at INFO level for picking up by external programs
    pub fn log(&self, msg: &str) {
        info!("{}", msg);
    }

    /// Cycle between known screens. Does not wrap from first to last
    pub fn cycle_screen(&mut self, direction: Direction) {
        if !self.screens.would_wrap(direction) {
            self.screens.cycle_focus(direction);
            let i = self.screens.focused().unwrap().wix;
            self.workspaces.focus(Selector::Index(i));
            self.conn.warp_cursor(None, self.screens.focused().unwrap());
            let wix = self.workspaces.focused_index();
            self.conn.set_current_workspace(wix);

            let i = self.screens.focused_index();
            run_hooks!(screen_change_hooks, self, i);
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

    /// Move the currently focused workspace to the next Screen in 'direction'
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
        self.running = false;
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
        }

        self.previous_workspace = active;
        run_hooks!(workspace_change_hooks, self, active, index);

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
            debug!("KILL_CLIENT for {}", id);
            self.conn.send_client_event(id, "WM_DELETE_WINDOW");
            self.conn.flush();

            self.remove_client(id);
            self.apply_layout(self.active_ws_index());
        }
    }

    /// Add a new workspace at `index`, shifting all workspaces with indices greater to the right.
    pub fn add_workspace(&mut self, index: usize, ws: Workspace) {
        self.workspaces.insert(index, ws)
    }

    /// Remove a Workspace from the WindowManager. All clients that were present on the removed
    /// workspace will be destroyed. WinId selectors will be ignored.
    pub fn remove_workspace(&mut self, selector: Selector<Workspace>) -> Option<Workspace> {
        if self.workspaces.len() == 1 {
            return None; // not allowed to remove the last workspace
        }

        self.workspaces.remove(selector).map(|ws| {
            ws.iter().for_each(|c| self.remove_client(*c));
            ws
        })
    }

    /// Get a reference to the first Workspace satisfying 'selector'. WinId selectors will return
    /// the workspace containing that Client if the client is known.
    pub fn workspace(&self, selector: Selector<Workspace>) -> Option<&Workspace> {
        self.workspaces.element(selector)
    }

    /// Get a mutable reference to the first Workspace satisfying 'selector'. WinId selectors will
    /// return the workspace containing that Client if the client is known.
    pub fn workspace_mut(&mut self, selector: Selector<Workspace>) -> Option<&mut Workspace> {
        self.workspaces.element_mut(selector)
    }

    /// Take a reference to the first Client found matching 'selector'
    pub fn client(&self, selector: Selector<Client>) -> Option<&Client> {
        match selector {
            Selector::Focused => self.focused_client(),
            Selector::WinId(id) => self.client_map.get(&id),
            Selector::Condition(f) => self.client_map.iter().find(|(_, v)| f(v)).map(|(_, v)| v),
            Selector::Index(i) => self.workspaces[self.active_ws_index()]
                .iter()
                .nth(i)
                .and_then(|id| self.client_map.get(id)),
        }
    }

    /// Take a mutable reference to the first Client found matching 'selector'
    pub fn client_mut(&mut self, selector: Selector<Client>) -> Option<&mut Client> {
        match selector {
            Selector::Focused => self.focused_client_mut(),
            Selector::WinId(id) => self.client_map.get_mut(&id),
            Selector::Condition(f) => self
                .client_map
                .iter_mut()
                .find(|(_, v)| f(v))
                .map(|(_, v)| v),
            Selector::Index(i) => match self.workspaces[self.active_ws_index()].iter().nth(i) {
                Some(id) => self.client_map.get_mut(id),
                None => None,
            },
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

    fn wm_with_mock_conn<'a>(layouts: Vec<Layout>, conn: &'a MockXConn) -> WindowManager<'a> {
        let mut conf = Config::default();
        conf.layouts = layouts;
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
        let conn = MockXConn::new(test_screens(), vec![]);
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
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0);
        wm.kill_client();

        assert_eq!(wm.workspaces[0].len(), 0);
    }

    #[test]
    fn kill_client_kills_focused_not_first() {
        let conn = MockXConn::new(test_screens(), vec![]);
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
        let conn = MockXConn::new(test_screens(), vec![]);
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
        let conn = MockXConn::new(test_screens(), vec![]);
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
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(1); // 20 -> ws::1
        wm.client_to_workspace(1); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(1);

        assert_eq!(wm.workspaces[1].focused_client(), Some(10));
    }

    #[test]
    fn x_focus_events_set_workspace_focus() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 5, 0); // focus on last client: 50
        wm.client_gained_focus(10);

        assert_eq!(wm.workspaces[0].focused_client(), Some(10));
    }

    #[test]
    fn dragging_clients_forward_from_index_0() {
        let conn = MockXConn::new(test_screens(), vec![]);
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
