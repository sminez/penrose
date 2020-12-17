//! Main logic for running Penrose
use crate::{
    bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
    client::Client,
    core::ring::{Direction, InsertPoint, Ring, Selector},
    data_types::{Change, Config, Point, Region, WinId},
    hooks,
    screen::Screen,
    workspace::Workspace,
    xconnection::{XConn, XEvent},
};

use nix::sys::signal::{signal, SigHandler, Signal};

use std::{cell::Cell, collections::HashMap};

// Relies on all hooks taking &mut WindowManager as the first arg.
macro_rules! run_hooks(
    ($method:ident, $_self:expr, $($arg:expr),*) => {
        let mut hooks = $_self.hooks.replace(vec![]);
        hooks.iter_mut().for_each(|h| h.$method($_self, $($arg),*));
        $_self.hooks.replace(hooks);
    };
);

/**
 * WindowManager is the primary struct / owner of the event loop for penrose.
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
    floating_classes: &'static [&'static str],
    focused_border: u32,
    unfocused_border: u32,
    border_px: u32,
    gap_px: u32,
    main_ratio_step: f32,
    show_bar: bool,
    bar_height: u32,
    top_bar: bool,
    hooks: Cell<Vec<Box<dyn hooks::Hook>>>,
    client_insert_point: InsertPoint,
    focused_client: Option<WinId>,
    running: bool,
}

impl<'a> WindowManager<'a> {
    /// Initialise a new window manager instance using an existing connection to the X server.
    pub fn init(config: Config, conn: &'a dyn XConn) -> WindowManager<'a> {
        let layouts = config.layouts.clone();
        let mut wm = WindowManager {
            conn,
            screens: Ring::new(vec![]),
            workspaces: Ring::new(vec![]),
            client_map: HashMap::new(),
            previous_workspace: 0,
            floating_classes: config.floating_classes,
            focused_border: config.focused_border,
            unfocused_border: config.unfocused_border,
            border_px: config.border_px,
            gap_px: config.gap_px,
            main_ratio_step: config.main_ratio_step,
            show_bar: config.show_bar,
            bar_height: config.bar_height,
            top_bar: config.top_bar,
            hooks: Cell::new(config.hooks),
            client_insert_point: InsertPoint::First,
            focused_client: None,
            running: false,
        };

        wm.workspaces = Ring::new(
            config
                .workspaces
                .iter()
                .map(|name| Workspace::new(*name, layouts.to_vec()))
                .collect(),
        );
        wm.detect_screens();
        conn.set_wm_properties(&config.workspaces);
        wm.conn.warp_cursor(None, &wm.screens[0]);

        wm
    }

    fn pad_region(&self, region: &Region, gapless: bool) -> Region {
        let gpx = if gapless { 0 } else { self.gap_px };
        let padding = 2 * (self.border_px + gpx);
        let (x, y, w, h) = region.values();
        Region::new(x + gpx, y + gpx, w - padding, h - padding)
    }

    fn apply_layout(&mut self, wix: usize) {
        let ws = match self.workspaces.get(wix) {
            Some(ws) => ws,
            None => {
                let len = self.workspaces.len();
                warn!("layout: wix out of bounds {} {}", wix, len);
                return;
            }
        };

        // Don't apply layouts if the workspace is not currently visible
        if let Some((i, s)) = self.indexed_screen_for_workspace(wix) {
            let lc = ws.layout_conf();
            if !lc.floating {
                let reg = s.region(self.show_bar);
                let arrange_result = ws.arrange(reg, &self.client_map);

                // Tile first then place floating clients on top
                for (id, region) in arrange_result.actions {
                    debug!("configuring {} with {:?}", id, region);
                    if let Some(region) = region {
                        let reg = self.pad_region(&region, lc.gapless);
                        self.conn.position_window(id, reg, self.border_px, false);
                        self.map_window_if_needed(id);
                    } else {
                        self.unmap_window_if_needed(id);
                    }
                }

                for id in arrange_result.floating {
                    self.conn.raise_window(id);
                }
            }
            run_hooks!(layout_applied, self, wix, i);
        }
    }

    fn map_window_if_needed(&mut self, id: WinId) {
        if let Some(c) = self.client_map.get_mut(&id) {
            if !c.mapped {
                c.mapped = true;
                self.conn.map_window(id);
            }
        }
    }

    fn unmap_window_if_needed(&mut self, id: WinId) {
        if let Some(c) = self.client_map.get_mut(&id) {
            if c.mapped {
                c.mapped = false;
                self.conn.unmap_window(id);
            }
        }
    }

    fn remove_client(&mut self, id: WinId) {
        match self.client_map.get(&id) {
            Some(client) => {
                self.workspaces
                    .get_mut(client.workspace())
                    .and_then(|ws| ws.remove_client(id));
                if let Some(c) = self.client_map.remove(&id) {
                    debug!("removing ref to client {} ({})", c.id(), c.class());
                }

                if self.focused_client == Some(id) {
                    self.focused_client = None;
                }
                run_hooks!(remove_client, self, id);
            }
            None => warn!("attempt to remove unknown client {}", id),
        }
    }

    fn update_x_workspace_details(&mut self) {
        let string_names: Vec<String> = self
            .workspaces
            .iter()
            .map(|ws| ws.name().to_string())
            .collect();
        let names: Vec<&str> = string_names.iter().map(|s| s.as_ref()).collect();

        self.conn.update_desktops(&names);
        run_hooks!(workspaces_updated, self, &names, self.active_ws_index());
    }

    /*
     * Helpers for indexing into WindowManager state
     */

    fn indexed_screen_for_workspace(&self, wix: usize) -> Option<(usize, &Screen)> {
        self.screens.iter().enumerate().find(|(_, s)| s.wix == wix)
    }

    fn set_screen_from_cursor(&mut self, cursor: Point) -> Option<&Screen> {
        self.focus_screen(&Selector::Condition(&|s: &Screen| s.contains(cursor)))
    }

    fn workspace_index_for_client(&mut self, id: WinId) -> Option<usize> {
        self.client_map.get(&id).map(|c| c.workspace())
    }

    fn active_ws_index(&self) -> usize {
        self.screens.focused().unwrap().wix
    }

    fn focus_screen(&mut self, sel: &Selector<Screen>) -> Option<&Screen> {
        let prev = self.screens.focused_index();
        self.screens.focus(sel);
        let new = self.screens.focused_index();

        if new != prev {
            run_hooks!(screen_change, self, new);
        }

        self.screens.focused()
    }

    fn focused_client(&self) -> Option<&Client> {
        self.focused_client
            .or_else(|| {
                self.workspaces
                    .get(self.active_ws_index())
                    .and_then(|ws| ws.focused_client())
            })
            .and_then(|id| self.client_map.get(&id))
    }

    fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focused_client
            .or_else(|| {
                self.workspaces
                    .get(self.active_ws_index())
                    .and_then(|ws| ws.focused_client())
            })
            .and_then(move |id| self.client_map.get_mut(&id))
    }

    fn client_gained_focus(&mut self, id: WinId) {
        let prev_focused = self.focused_client().map(|c| c.id());
        if let Some(id) = prev_focused {
            self.client_lost_focus(id)
        }

        self.conn.set_client_border_color(id, self.focused_border);
        self.conn.focus_client(id);

        if let Some(wix) = self.workspace_index_for_client(id) {
            if let Some(ws) = self.workspaces.get_mut(wix) {
                ws.focus_client(id);
                let prev_was_in_ws = prev_focused.map_or(false, |id| ws.clients().contains(&id));
                if ws.layout_conf().follow_focus && prev_was_in_ws {
                    self.apply_layout(wix);
                }
            }
        }

        self.focused_client = Some(id);
        run_hooks!(focus_change, self, id);
    }

    fn client_lost_focus(&self, id: WinId) {
        let color = self.unfocused_border;
        self.conn.set_client_border_color(id, color);
    }

    /**
     * main event loop for the window manager.
     * Everything is driven by incoming events from the X server with each event type being
     * mapped to a handler
     */
    pub fn grab_keys_and_run(
        &mut self,
        mut bindings: KeyBindings,
        mut mouse_bindings: MouseBindings,
    ) {
        // ignore SIGCHILD and allow child / inherited processes to be inherited by pid1
        unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) }.unwrap();

        self.conn.grab_keys(&bindings, &mouse_bindings);
        self.focus_workspace(&Selector::Index(0));
        run_hooks!(startup, self,);
        self.running = true;

        while self.running {
            if let Some(event) = self.conn.wait_for_event() {
                debug!("got XEvent: {:?}", event);
                match event {
                    XEvent::MouseEvent(e) => self.handle_mouse_event(e, &mut mouse_bindings),
                    XEvent::KeyPress(code) => self.handle_key_press(code, &mut bindings),
                    XEvent::MapRequest { id, ignore } => self.handle_map_request(id, ignore),
                    XEvent::Enter { id, rpt, wpt } => self.handle_enter_notify(id, rpt, wpt),
                    XEvent::Leave { id, rpt, wpt } => self.handle_leave_notify(id, rpt, wpt),
                    XEvent::Destroy { id } => self.handle_destroy_notify(id),
                    XEvent::ScreenChange => self.handle_screen_change(),
                    XEvent::RandrNotify => self.detect_screens(),
                    XEvent::ConfigureNotify { id, r, is_root } => {
                        self.handle_configure_notify(id, r, is_root)
                    }
                    XEvent::PropertyNotify { id, atom, is_root } => {
                        self.handle_property_notify(id, &atom, is_root)
                    }
                    XEvent::ClientMessage { id, dtype, data } => {
                        self.handle_client_message(id, &dtype, &data)
                    }
                }
                run_hooks!(event_handled, self,);
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
    fn handle_key_press(&mut self, key_code: KeyCode, bindings: &mut KeyBindings) {
        debug!("handling key code: {:?}", key_code);
        if let Some(action) = bindings.get_mut(&key_code) {
            action(self); // ignoring Child handlers and SIGCHILD
        }
    }

    fn handle_mouse_event(&mut self, e: MouseEvent, bindings: &mut MouseBindings) {
        debug!("handling mouse event: {:?} {:?}", e.state, e.kind);
        if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
            action(self, &e); // ignoring Child handlers and SIGCHILD
        }
    }

    fn handle_map_request(&mut self, id: WinId, override_redirect: bool) {
        if override_redirect || self.client_map.contains_key(&id) {
            return;
        }

        let wm_class = match self.conn.str_prop(id, "WM_CLASS") {
            Ok(s) => s.split('\0').collect::<Vec<&str>>()[0].into(),
            Err(_) => String::new(),
        };

        let wm_name = match self.conn.str_prop(id, "WM_NAME") {
            Ok(s) => s,
            Err(_) => String::from("n/a"),
        };

        let floating = self.conn.window_should_float(id, self.floating_classes);
        let mut client = Client::new(id, wm_name, wm_class, self.active_ws_index(), floating);
        run_hooks!(new_client, self, &mut client);
        let wix = client.workspace();

        if client.wm_managed {
            self.add_client_to_workspace(wix, id);
        }

        if client.floating {
            if let Ok(default_position) = self.conn.window_geometry(id) {
                let (mut x, mut y, w, h) = default_position.values();
                if let Some((_, s)) = self.indexed_screen_for_workspace(wix) {
                    let (sx, sy, _, _) = s.region(self.show_bar).values();
                    x = if x < sx { sx } else { x };
                    y = if y < sy { sy } else { y };
                    let reg = self.pad_region(&Region::new(x, y, w, h), false);
                    self.conn.position_window(id, reg, self.border_px, false);
                }
            }
        }

        self.client_map.insert(id, client);
        self.conn.set_client_workspace(id, wix);

        if wix == self.active_ws_index() {
            self.conn.mark_new_window(id);
            self.conn.focus_client(id);
            self.client_gained_focus(id);

            self.apply_layout(wix);
            self.map_window_if_needed(id);

            let s = self.screens.focused().unwrap();
            self.conn.warp_cursor(Some(id), s);
        }
    }

    fn add_client_to_workspace(&mut self, wix: usize, id: WinId) {
        let cip = self.client_insert_point;
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.add_client(id, &cip)
        };
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

    fn handle_configure_notify(&mut self, _: WinId, _: Region, is_root: bool) {
        if is_root {
            self.detect_screens()
        }
    }

    fn handle_screen_change(&mut self) {
        self.set_screen_from_cursor(self.conn.cursor_position());
        let wix = self.screens.focused().unwrap().wix;
        self.workspaces.focus(&Selector::Index(wix));
    }

    fn handle_destroy_notify(&mut self, win_id: WinId) {
        debug!("DESTROY_NOTIFY for {}", win_id);
        self.remove_client(win_id);
        self.apply_layout(self.active_ws_index());
    }

    fn handle_property_notify(&mut self, id: WinId, atom: &str, is_root: bool) {
        if atom == "WM_NAME" || atom == "_NET_WM_NAME" {
            if let Ok(name) = self.conn.str_prop(id, atom) {
                if let Some(c) = self.client_map.get_mut(&id) {
                    c.set_name(&name)
                }
                run_hooks!(client_name_updated, self, id, &name, is_root);
            }
        }
    }

    fn handle_client_message(&mut self, id: WinId, dtype: &str, data: &[usize]) {
        if dtype == "_NET_WM_STATE" {
            let full_screen = self.conn.intern_atom("_NET_WM_STATE_FULLSCREEN").unwrap() as usize;
            if data.get(1) == Some(&full_screen) || data.get(2) == Some(&full_screen) {
                let client_is_fullscreen = match self.client_map.get(&id) {
                    None => return, // unknown client
                    Some(c) => c.fullscreen,
                };
                // _NET_WM_STATE_ADD == 1, _NET_WM_STATE_TOGGLE == 2
                let should_fullscreen = [1, 2].contains(&data[0]) && !client_is_fullscreen;
                self.set_fullscreen(id, should_fullscreen, client_is_fullscreen);
            }
        }
    }

    fn set_fullscreen(&mut self, id: WinId, should_fullscreen: bool, client_is_fullscreen: bool) {
        if should_fullscreen && !client_is_fullscreen {
            self.conn.toggle_client_fullscreen(id, client_is_fullscreen);
            if let Some(ws) = self.workspaces.get(self.active_ws_index()) {
                ws.clients().iter().for_each(|&i| {
                    if i != id {
                        self.unmap_window_if_needed(i)
                    }
                });
            }
            let r = self.screen(&Selector::Focused).unwrap().region(false);
            self.conn.position_window(id, r, 0, false);
            self.map_window_if_needed(id);
            if let Some(c) = self.client_map.get_mut(&id) {
                c.fullscreen = true
            };
        } else if !should_fullscreen && client_is_fullscreen {
            self.conn.toggle_client_fullscreen(id, client_is_fullscreen);
            if let Some(ws) = self.workspaces.get(self.active_ws_index()) {
                ws.clients().iter().for_each(|&i| {
                    if i != id {
                        self.map_window_if_needed(i)
                    }
                });
            }
            self.apply_layout(self.active_ws_index());
            if let Some(c) = self.client_map.get_mut(&id) {
                c.fullscreen = false
            };
        }
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

        let regions: Vec<_> = self.screens.iter().map(|s| s.region(false)).collect();
        run_hooks!(screens_updated, self, &regions);
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
            self.workspaces.focus(&Selector::Index(i));
            self.conn.warp_cursor(None, self.screens.focused().unwrap());
            let wix = self.workspaces.focused_index();
            self.conn.set_current_workspace(wix);

            let i = self.screens.focused_index();
            run_hooks!(screen_change, self, i);
        }
    }

    /**
     * Cycle between workspaces on the current Screen. This will pull workspaces
     * to the screen if they are currently displayed on another screen.
     */
    pub fn cycle_workspace(&mut self, direction: Direction) {
        self.workspaces.cycle_focus(direction);
        let i = self.workspaces.focused_index();
        self.focus_workspace(&Selector::Index(i));
    }

    /// Move the currently focused workspace to the next Screen in 'direction'
    pub fn drag_workspace(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        self.cycle_screen(direction);
        self.focus_workspace(&Selector::Index(wix)); // focus_workspace will pull it to the new screen
    }

    /// Cycle between Clients for the active Workspace
    pub fn cycle_client(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        let res = self
            .workspaces
            .get_mut(wix)
            .and_then(|ws| ws.cycle_client(direction));
        if let Some((prev, new)) = res {
            self.client_lost_focus(prev);
            self.client_gained_focus(new);
            let screen = self.screens.focused().unwrap();
            self.conn.warp_cursor(Some(new), screen);
        }
    }

    /// Rotate the client stack on the active Workspace
    pub fn rotate_clients(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.rotate_clients(direction)
        };
    }

    /// Move the focused Client through the stack of Clients on the active Workspace
    pub fn drag_client(&mut self, direction: Direction) {
        if let Some(id) = self.focused_client().map(|c| c.id()) {
            let wix = self.active_ws_index();
            self.workspaces
                .get_mut(wix)
                .and_then(|ws| ws.drag_client(direction));
            self.apply_layout(wix);
            self.client_gained_focus(id);
            self.conn
                .warp_cursor(Some(id), self.screens.focused().unwrap());
        }
    }

    /// Cycle between Layouts for the active Workspace
    pub fn cycle_layout(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.cycle_layout(direction);
        }
        run_hooks!(layout_change, self, wix, self.active_screen_index());
        self.apply_layout(wix);
    }

    /// Increase or decrease the number of clients in the main area by 1
    pub fn update_max_main(&mut self, change: Change) {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.update_max_main(change)
        };
        self.apply_layout(wix);
    }

    /// Increase or decrease the current Layout main_ratio by main_ratio_step
    pub fn update_main_ratio(&mut self, change: Change) {
        let step = self.main_ratio_step;
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.update_main_ratio(change, step)
        }
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
        self.layout_symbol(self.active_ws_index())
    }

    fn layout_symbol(&self, wix: usize) -> &str {
        match self.workspaces.get(wix) {
            Some(ws) => ws.layout_symbol(),
            None => "???",
        }
    }

    /// Set the root X window name. Useful for exposing information to external programs
    pub fn set_root_window_name(&self, s: &str) {
        self.conn.set_root_window_name(s);
    }

    /// Set the insert point for new clients. Default is to insert at index 0.
    pub fn set_client_insert_point(&mut self, cip: InsertPoint) {
        self.client_insert_point = cip;
    }

    /**
     * Set the displayed workspace for the focused screen to be `index` in the list of
     * workspaces passed at `init`. This will panic if the index passed is out of
     * bounds which is only possible if you manually bind an action to this with an
     * invalid index. You should almost always be using the `gen_keybindings!` macro
     * to set up your keybindings so this is not normally an issue.
     */
    pub fn focus_workspace(&mut self, selector: &Selector<Workspace>) {
        let active_ws = Selector::Index(self.screens.focused().unwrap().wix);
        if self.workspaces.equivalent_selectors(selector, &active_ws) {
            return;
        }

        if let Some(index) = self.workspaces.index(selector) {
            let active = self.active_ws_index();
            self.previous_workspace = active;

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

                    let ws = self.workspaces.get(index);
                    if let Some(id) = ws.and_then(|ws| ws.focused_client()) {
                        self.client_gained_focus(id)
                    };

                    self.workspaces.focus(&Selector::Index(index));
                    run_hooks!(workspace_change, self, active, index);
                    return;
                }
            }

            // target not currently displayed so unmap what we currently have
            // displayed and replace it with the target workspace
            if let Some(ws) = self.workspaces.get(active) {
                ws.clients()
                    .iter()
                    .for_each(|id| self.unmap_window_if_needed(*id));
            }

            if let Some(ws) = self.workspaces.get(index) {
                ws.clients()
                    .iter()
                    .for_each(|id| self.map_window_if_needed(*id));
            }

            self.screens.focused_mut().unwrap().wix = index;
            self.apply_layout(index);
            self.conn.set_current_workspace(index);

            let ws = self.workspaces.get(index);
            if let Some(id) = ws.and_then(|ws| ws.focused_client()) {
                self.client_gained_focus(id)
            };

            self.workspaces.focus(&Selector::Index(index));
            run_hooks!(workspace_change, self, active, index);
        }
    }

    /// Switch focus back to the last workspace that had focus.
    pub fn toggle_workspace(&mut self) {
        self.focus_workspace(&Selector::Index(self.previous_workspace));
    }

    /// Move the focused client to the workspace matching 'selector'.
    pub fn client_to_workspace(&mut self, selector: &Selector<Workspace>) {
        let active_ws = Selector::Index(self.screens.focused().unwrap().wix);
        if self.workspaces.equivalent_selectors(&selector, &active_ws) {
            return;
        }

        if let Some(index) = self.workspaces.index(&selector) {
            let res = self
                .workspaces
                .get_mut(self.active_ws_index())
                .and_then(|ws| ws.remove_focused_client());

            if let Some(id) = res {
                self.add_client_to_workspace(index, id);
                if let Some(c) = self.client_map.get_mut(&id) {
                    c.set_workspace(index)
                };
                self.conn.set_client_workspace(id, index);
                self.apply_layout(self.active_ws_index());

                // layout & focus the screen we just landed on if the workspace is displayed
                // otherwise unmap the window because we're no longer visible
                if self.screens.iter().any(|s| s.wix == index) {
                    self.apply_layout(index);
                    let s = self.screens.focused().unwrap();
                    self.conn.warp_cursor(Some(id), s);
                    self.focus_screen(&Selector::Index(self.active_screen_index()));
                } else {
                    self.unmap_window_if_needed(id);
                }
            };
        }
    }

    /// Move the focused client to the active workspace on the screen matching 'selector'.
    pub fn client_to_screen(&mut self, selector: &Selector<Screen>) {
        let i = match self.screen(selector) {
            Some(s) => s.wix,
            None => return,
        };
        self.client_to_workspace(&Selector::Index(i));
    }

    /// Toggle the fullscreen state of the given client ID
    pub fn toggle_client_fullscreen(&mut self, selector: &Selector<Client>) {
        let (id, client_is_fullscreen) = match self.client(selector) {
            None => return, // unknown client
            Some(c) => (c.id(), c.fullscreen),
        };
        self.set_fullscreen(id, !client_is_fullscreen, client_is_fullscreen);
    }

    /// Kill the focused client window.
    pub fn kill_client(&mut self) {
        let id = self.conn.focused_client();
        self.conn.send_client_event(id, "WM_DELETE_WINDOW").unwrap();
        self.conn.flush();

        self.remove_client(id);
        self.apply_layout(self.active_ws_index());
    }

    /// Get a reference to the first Screen satisfying 'selector'. WinId selectors will return
    /// the screen containing that Client if the client is known.
    /// NOTE: It is not possible to get a mutable reference to a Screen.
    pub fn screen(&self, selector: &Selector<Screen>) -> Option<&Screen> {
        if let Selector::WinId(id) = selector {
            self.client_map.get(&id).and_then(|c| {
                self.screens
                    .element(&Selector::Condition(&|s| s.wix == c.workspace()))
            })
        } else {
            self.screens.element(&selector)
        }
    }

    /// The currently focused workspace indices being shown on each screen
    pub fn focused_workspaces(&self) -> Vec<usize> {
        self.screens.iter().map(|s| s.wix).collect()
    }

    /// Add a new workspace at `index`, shifting all workspaces with indices greater to the right.
    pub fn add_workspace(&mut self, index: usize, ws: Workspace) {
        self.workspaces.insert(index, ws);
        self.update_x_workspace_details();
    }

    /// Add a new workspace at the end of the current workspace list
    pub fn push_workspace(&mut self, ws: Workspace) {
        self.workspaces.push(ws);
        self.update_x_workspace_details();
    }

    /// Remove a Workspace from the WindowManager. All clients that were present on the removed
    /// workspace will be destroyed. WinId selectors will be ignored.
    pub fn remove_workspace(&mut self, selector: &Selector<Workspace>) -> Option<Workspace> {
        if self.workspaces.len() == 1 {
            return None; // not allowed to remove the last workspace
        }

        let ws = self.workspaces.remove(&selector)?;
        ws.iter().for_each(|c| self.remove_client(*c));

        // Focus the workspace before the one we just removed. There is always at least one
        // workspace before this one due to the guard above.
        let ix = self.screens.focused()?.wix - 1;
        self.focus_workspace(&Selector::Index(ix));

        self.update_x_workspace_details();
        Some(ws)
    }

    /// Get a reference to the first Workspace satisfying 'selector'. WinId selectors will return
    /// the workspace containing that Client if the client is known.
    pub fn workspace(&self, selector: &Selector<Workspace>) -> Option<&Workspace> {
        if let Selector::WinId(id) = selector {
            self.client_map
                .get(&id)
                .and_then(|c| self.workspaces.get(c.workspace()))
        } else {
            self.workspaces.element(&selector)
        }
    }

    /// Get a mutable reference to the first Workspace satisfying 'selector'. WinId selectors will
    /// return the workspace containing that Client if the client is known.
    pub fn workspace_mut(&mut self, selector: &Selector<Workspace>) -> Option<&mut Workspace> {
        if let Selector::WinId(id) = selector {
            if let Some(wix) = self.client_map.get(&id).map(|c| c.workspace()) {
                self.workspaces.get_mut(wix)
            } else {
                None
            }
        } else {
            self.workspaces.element_mut(&selector)
        }
    }

    /// Get a vector of references to Workspaces satisfying 'selector'. WinId selectors will return
    /// a vector with the workspace containing that Client if the client is known. Otherwise an
    /// empty vector will be returned.
    pub fn all_workspaces(&self, selector: &Selector<Workspace>) -> Vec<&Workspace> {
        if let Selector::WinId(id) = selector {
            self.client_map
                .get(&id)
                .and_then(|c| self.workspaces.get(c.workspace()))
                .into_iter()
                .collect()
        } else {
            self.workspaces.all_elements(&selector)
        }
    }

    /// Get a vector of mutable references to Workspaces satisfying 'selector'. WinId selectors will
    /// return a vector with the workspace containing that Client if the client is known. Otherwise
    /// an empty vector will be returned.
    pub fn all_workspaces_mut(&mut self, selector: &Selector<Workspace>) -> Vec<&mut Workspace> {
        if let Selector::WinId(id) = selector {
            if let Some(wix) = self.client_map.get(&id).map(|c| c.workspace()) {
                self.workspaces.all_elements_mut(&Selector::Index(wix))
            } else {
                return vec![];
            }
        } else {
            self.workspaces.all_elements_mut(&selector)
        }
    }

    /// Set the name of the selected Workspace
    pub fn set_workspace_name(&mut self, name: impl Into<String>, selector: Selector<Workspace>) {
        if let Some(ws) = self.workspaces.element_mut(&selector) {
            ws.set_name(name)
        };
        self.update_x_workspace_details();
    }

    /// Take a reference to the first Client found matching 'selector'
    pub fn client(&self, selector: &Selector<Client>) -> Option<&Client> {
        match selector {
            Selector::Focused => self.focused_client(),
            Selector::WinId(id) => self.client_map.get(&id),
            Selector::Condition(f) => self.client_map.iter().find(|(_, v)| f(v)).map(|(_, v)| v),
            Selector::Index(i) => self
                .workspaces
                .get(self.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
                .and_then(|id| self.client_map.get(id)),
        }
    }

    /// Take a mutable reference to the first Client found matching 'selector'
    pub fn client_mut(&mut self, selector: &Selector<Client>) -> Option<&mut Client> {
        match selector {
            Selector::Focused => self.focused_client_mut(),
            Selector::WinId(id) => self.client_map.get_mut(&id),
            Selector::Condition(f) => self
                .client_map
                .iter_mut()
                .find(|(_, v)| f(v))
                .map(|(_, v)| v),
            Selector::Index(i) => match self
                .workspaces
                .get(self.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
            {
                Some(id) => self.client_map.get_mut(id),
                None => None,
            },
        }
    }

    /// Get a vector of references to the Clients found matching 'selector'.
    /// The resulting vector is sorted by Client id.
    pub fn all_clients(&self, selector: &Selector<Client>) -> Vec<&Client> {
        match selector {
            Selector::Focused => self.focused_client().into_iter().collect(),
            Selector::WinId(id) => self.client_map.get(&id).into_iter().collect(),
            Selector::Condition(f) => {
                let mut clients = self
                    .client_map
                    .iter()
                    .flat_map(|(_, v)| if f(v) { Some(v) } else { None })
                    .collect::<Vec<_>>();
                clients.sort_by_key(|&a| a.id());
                clients
            }
            Selector::Index(i) => self
                .workspaces
                .get(self.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
                .and_then(|id| self.client_map.get(id))
                .into_iter()
                .collect(),
        }
    }

    /// Get a vector of mutable references to the Clients found matching 'selector'.
    /// The resulting vector is sorted by Client id.
    pub fn all_clients_mut(&mut self, selector: &Selector<Client>) -> Vec<&mut Client> {
        match selector {
            Selector::Focused => self.focused_client_mut().into_iter().collect(),
            Selector::WinId(id) => self.client_map.get_mut(&id).into_iter().collect(),
            Selector::Condition(f) => {
                let mut clients = self
                    .client_map
                    .iter_mut()
                    .flat_map(|(_, v)| if f(v) { Some(v) } else { None })
                    .collect::<Vec<_>>();
                clients.sort_by_key(|a| a.id());
                clients
            }
            Selector::Index(i) => match self
                .workspaces
                .get(self.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
            {
                Some(id) => self.client_map.get_mut(id).into_iter().collect(),
                None => vec![],
            },
        }
    }

    /// The number of detected screens currently being tracked by the WindowManager.
    pub fn n_screens(&self) -> usize {
        self.screens.len()
    }

    /// The current effective screen size of the target screen. Effective screen size is the
    /// physical screen size minus any space reserved for a status bar.
    pub fn screen_size(&self, screen_index: usize) -> Option<Region> {
        self.screens
            .get(screen_index)
            .map(|s| s.region(self.show_bar))
    }

    /// Position an individual client on the display. (x,y) coordinates are absolute (i.e. relative
    /// to the root window not any individual screen).
    pub fn position_client(&self, id: WinId, region: Region, stack_above: bool) {
        self.conn
            .position_window(id, region, self.border_px, stack_above);
    }

    /// Make the Client with ID 'id' visible at its last known position.
    pub fn show_client(&mut self, id: WinId) {
        self.map_window_if_needed(id);
    }

    /// Hide the Client with ID 'id'.
    pub fn hide_client(&mut self, id: WinId) {
        self.unmap_window_if_needed(id);
    }

    /// Layout the workspace currently shown on the given screen index.
    pub fn layout_screen(&mut self, screen_index: usize) {
        if let Some(wix) = self.screens.get(screen_index).map(|s| s.wix) {
            self.apply_layout(wix)
        }
    }

    /// An index into the WindowManager known screens for the screen that is currently focused
    pub fn active_screen_index(&self) -> usize {
        self.screens.focused_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ring::Direction::*;
    use crate::data_types::*;
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
        vec![
            Screen::new(Region::new(0, 0, 1366, 768), 0),
            Screen::new(Region::new(1366, 0, 1366, 768), 0),
        ]
    }

    fn add_n_clients(wm: &mut WindowManager, n: usize, offset: usize) {
        for i in 0..n {
            wm.handle_map_request(10 * (i + offset + 1) as u32, false);
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
        wm.focus_workspace(&Selector::Index(1));
        add_n_clients(&mut wm, 2, 3);
        assert_eq!(wm.workspaces[1].len(), 2);
        assert_eq!(wm.workspaces[1].focused_client(), Some(50));

        // switch back: clients should be the same, same client should have focus
        wm.focus_workspace(&Selector::Index(0));
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

        let ids: Vec<WinId> = wm.workspaces[0].iter().cloned().collect();
        assert_eq!(ids, vec![50, 30, 20, 10]);
        assert_eq!(wm.workspaces[0].focused_client(), Some(30));
    }

    #[test]
    fn moving_then_deleting_clients() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0);
        wm.client_to_workspace(&Selector::Index(1));
        wm.client_to_workspace(&Selector::Index(1));
        wm.focus_workspace(&Selector::Index(1));
        wm.kill_client();

        // should have removed first client on ws::1 (last sent from ws::0)
        assert_eq!(wm.workspaces[1].iter().collect::<Vec<&WinId>>(), vec![&20]);
    }

    #[test]
    fn client_to_workspace_inserts_at_head() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(&Selector::Index(1)); // 20 -> ws::1
        wm.client_to_workspace(&Selector::Index(1)); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(&Selector::Index(1));

        assert_eq!(
            wm.workspaces[1].iter().collect::<Vec<&WinId>>(),
            vec![&10, &20]
        );
    }

    #[test]
    fn client_to_workspace_sets_focus() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(&Selector::Index(1)); // 20 -> ws::1
        wm.client_to_workspace(&Selector::Index(1)); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(&Selector::Index(1));

        assert_eq!(wm.workspaces[1].focused_client(), Some(10));
    }

    #[test]
    fn client_to_invalid_workspace_is_noop() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
        wm.client_to_workspace(&Selector::Index(42));
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
    }

    #[test]
    fn client_to_screen_sets_correct_workspace() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        wm.client_to_screen(&Selector::Index(1));
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(1));
    }

    #[test]
    fn client_to_invalid_screen_is_noop() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
        wm.client_to_screen(&Selector::Index(5));
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
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
    fn focus_workspace_sets_focus_in_ring() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        assert_eq!(wm.workspaces.focused_index(), 0);
        assert_eq!(wm.workspaces.focused_index(), wm.active_ws_index());
        wm.focus_workspace(&Selector::Index(3));
        assert_eq!(wm.workspaces.focused_index(), 3);
        assert_eq!(wm.workspaces.focused_index(), wm.active_ws_index());
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

    #[test]
    fn getting_all_clients_on_workspace() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);

        add_n_clients(&mut wm, 3, 0);
        wm.focus_workspace(&Selector::Index(1));
        add_n_clients(&mut wm, 2, 3);

        let ws_0 = Selector::Condition(&|c: &Client| c.workspace() == 0);
        let ws_1 = Selector::Condition(&|c: &Client| c.workspace() == 1);

        assert_eq!(wm.all_clients(&ws_0).len(), 3);
        assert_eq!(wm.all_clients_mut(&ws_1).len(), 2);
    }

    #[test]
    fn getting_all_workspaces_of_window() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);

        add_n_clients(&mut wm, 3, 0);
        wm.focus_workspace(&Selector::Index(1));
        add_n_clients(&mut wm, 2, 3);

        assert_eq!(wm.all_workspaces(&Selector::WinId(40))[0].name(), "2");
        assert_eq!(wm.all_workspaces_mut(&Selector::WinId(10))[0].name(), "1");
    }

    #[test]
    fn selector_screen() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0);

        assert_eq!(wm.screen(&Selector::Focused), wm.screens.focused());
        assert_eq!(wm.screen(&Selector::Index(1)), wm.screens.get(1));
        assert_eq!(wm.screen(&Selector::WinId(10)), wm.screens.get(0));
        assert_eq!(
            wm.screen(&Selector::Condition(&|s| s.wix == 1)),
            wm.screens.get(1)
        );
    }

    #[test]
    fn selector_workspace() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 1, 0);

        assert_eq!(wm.workspace(&Selector::Focused), wm.workspaces.focused());
        assert_eq!(wm.workspace(&Selector::Index(1)), wm.workspaces.get(1));
        assert_eq!(wm.workspace(&Selector::WinId(10)), wm.workspaces.get(0));
        assert_eq!(
            wm.workspace(&Selector::Condition(&|w| w.name() == "3")),
            wm.workspaces.get(2)
        );
    }

    #[test]
    fn selector_client() {
        let conn = MockXConn::new(test_screens(), vec![]);
        let mut wm = wm_with_mock_conn(test_layouts(), &conn);
        add_n_clients(&mut wm, 4, 0);

        assert_eq!(wm.client(&Selector::Focused), wm.client_map.get(&40));
        assert_eq!(wm.client(&Selector::Index(2)), wm.client_map.get(&20));
        assert_eq!(wm.client(&Selector::WinId(30)), wm.client_map.get(&30));
        assert_eq!(
            wm.client(&Selector::Condition(&|c| c.id() == 10)),
            wm.client_map.get(&10)
        );
    }
}
