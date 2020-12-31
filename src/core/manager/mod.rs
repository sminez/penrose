//! The main user API and control logic for Penrose.
use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
        client::Client,
        config::Config,
        data_types::{Change, Point, Region, WinId},
        hooks::Hook,
        layout::LayoutFunc,
        ring::{Direction, InsertPoint, Ring, Selector},
        screen::Screen,
        workspace::Workspace,
        xconnection::{Atom, XConn},
    },
    Result,
};

use nix::sys::signal::{signal, SigHandler, Signal};

use std::{cell::Cell, collections::HashMap, fmt};

mod event;
mod util;

#[doc(inline)]
pub use event::EventAction;

use event::{process_next_event, WmState};

// Relies on all hooks taking &mut WindowManager as the first arg.
macro_rules! run_hooks {
    ($method:ident, $_self:expr, $($arg:expr),*) => {
        debug!("Running {} hooks", stringify!($method));
        let mut hooks = $_self.hooks.replace(vec![]);
        hooks.iter_mut().for_each(|h| h.$method($_self, $($arg),*));
        $_self.hooks.replace(hooks);
    };
}

// NOTE: Helpers for stubbing out non-serializable state from the WindowManager
//       when deserializing with serde. WindowManager::hydrate_and_init MUST
//       be called following serialization otherwise the WindowManager will
//       panic on init.

#[cfg(feature = "serde")]
struct StubConn {}
#[cfg(feature = "serde")]
impl crate::core::xconnection::StubXConn for StubConn {
    // NOTE: panicking here as it is the first XConn method to be called in __init
    fn mock_current_outputs(&self) -> Vec<Screen> {
        panic!("StubConn is not usable as a real XConn impl: call hydrate_and_init instead");
    }

    // NOTE: panicking here as it is the first XConn method to be called in grab_keys_and_run
    fn mock_wait_for_event(&self) -> Option<crate::core::xconnection::XEvent> {
        panic!("StubConn is not usable as a real XConn impl: call hydrate_and_init instead");
    }
}

#[cfg(feature = "serde")]
fn default_conn() -> Box<dyn XConn> {
    Box::new(StubConn {})
}

#[cfg(feature = "serde")]
fn default_hooks() -> Cell<Vec<Box<dyn Hook>>> {
    Cell::new(Vec::new())
}

/**
 * WindowManager is the primary struct / owner of the event loop for penrose.
 * It handles most (if not all) of the communication with XCB and responds to
 * X events served over the embedded connection. User input bindings are parsed
 * and bound on init and then triggered via grabbed X events in the main loop
 * along with everything else.
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WindowManager {
    #[cfg_attr(feature = "serde", serde(skip, default = "default_conn"))]
    conn: Box<dyn XConn>,
    config: Config,
    screens: Ring<Screen>,
    workspaces: Ring<Workspace>,
    client_map: HashMap<WinId, Client>,
    #[cfg_attr(feature = "serde", serde(skip, default = "default_hooks"))]
    hooks: Cell<Vec<Box<dyn Hook>>>,
    previous_workspace: usize,
    client_insert_point: InsertPoint,
    focused_client: Option<WinId>,
    running: bool,
}

impl fmt::Debug for WindowManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowManager")
            .field("conn", &stringify!(self.conn))
            .field("config", &self.config)
            .field("screens", &self.screens)
            .field("workspaces", &self.workspaces)
            .field("client_map", &self.client_map)
            .field("hooks", &stringify!(self.hooks))
            .field("previous_workspace", &self.previous_workspace)
            .field("client_insert_point", &self.client_insert_point)
            .field("focused_client", &self.focused_client)
            .field("running", &self.running)
            .finish()
    }
}

impl WindowManager {
    /**
     * Construct a new window manager instance using a chosen [XConn] backed to communicate
     * with the X server.
     */
    pub fn new(config: Config, conn: Box<dyn XConn>, hooks: Vec<Box<dyn Hook>>) -> WindowManager {
        let layouts = config.layouts.clone();

        debug!("Building initial workspaces");
        let workspaces = config
            .workspaces
            .iter()
            .map(|name| Workspace::new(name, layouts.to_vec()))
            .collect();

        Self {
            conn,
            config,
            screens: Ring::new(vec![]),
            workspaces,
            client_map: HashMap::new(),
            previous_workspace: 0,
            hooks: Cell::new(hooks),
            client_insert_point: InsertPoint::First,
            focused_client: None,
            running: false,
        }
    }

    /// Restore missing state following serde deserialization
    pub fn hydrate_and_init(
        &mut self,
        conn: Box<dyn XConn>,
        hooks: Vec<Box<dyn Hook>>,
        layout_funcs: HashMap<&str, LayoutFunc>,
    ) -> Result<()> {
        self.conn = conn;
        self.hooks.set(hooks);
        self.workspaces
            .iter_mut()
            .map(|w| w.restore_layout_functions(&layout_funcs))
            .collect::<Result<()>>()?;

        util::validate_hydrated_wm_state(self)?;
        self.init();
        Ok(())
    }

    /**
     * This initialises the [WindowManager] internal state but does not start processing any
     * events from the X server. If you need to perform any custom setup logic with the
     * [WindowManager] itself, it should be run after calling this method and before
     * [WindowManager::grab_keys_and_run].
     */
    pub fn init(&mut self) {
        debug!("Attempting initial screen detection");
        self.detect_screens();

        debug!("Setting EWMH properties");
        self.conn
            .set_wm_properties(str_slice!(self.config.workspaces));

        debug!("Forcing cursor to first screen");
        self.conn.warp_cursor(None, &self.screens[0]);
    }

    // Subset of current immutable state that is needed for processing XEvents into EventActions
    fn current_state(&self) -> WmState<'_> {
        WmState {
            client_map: &self.client_map,
            focused_client: self.focused_client,
            full_screen_atom: self
                .conn
                .intern_atom(Atom::NetWmStateFullscreen.as_ref())
                .unwrap() as usize,
        }
    }

    // Each XEvent from the XConn can result in multiple EventActions that need processing
    // depending on the current WindowManager state.
    fn handle_event_action(
        &mut self,
        action: EventAction,
        key_bindings: &mut KeyBindings,
        mouse_bindings: &mut MouseBindings,
    ) -> Result<()> {
        debug!("Handling event action: {:?}", action);
        match action {
            EventAction::ClientFocusGained(id) => self.client_gained_focus(id),
            EventAction::ClientFocusLost(id) => self.client_lost_focus(id),
            EventAction::ClientNameChanged(id) => self.client_name_changed(id)?,
            EventAction::DestroyClient(id) => self.remove_client(id),
            EventAction::DetectScreens => self.detect_screens(),
            EventAction::MapWindow(id) => self.handle_map_request(id)?,
            EventAction::RunKeyBinding(k) => self.run_key_binding(k, key_bindings),
            EventAction::RunMouseBinding(e) => self.run_mouse_binding(e, mouse_bindings),
            EventAction::SetScreenFromPoint(p) => self.set_screen_from_point(p),
            EventAction::ToggleClientFullScreen(id, should_fullscreen) => {
                self.set_fullscreen(id, should_fullscreen);
            }
            EventAction::UnknownPropertyChange(..) => {}
        }
        Ok(())
    }

    /**
     * This is the main event loop for the [WindowManager].
     *
     * The [XConn::wait_for_event] method is called to fetch the next event from the X server,
     * after which it is processed into a set of internal EventActions which are then processed
     * by the [WindowManager] to update state and perform actions. This method is an infinite
     * loop until the [WindowManager::exit] method is called, which triggers [XConn::cleanup]
     * before exiting the loop. You can provide any additional teardown logic you need your
     * main.rs after the call to [WindowManager::grab_keys_and_run] and all internal state
     * will still be accessible (though methods requiring the use of the [XConn] will fail.
     */
    pub fn grab_keys_and_run(
        &mut self,
        mut key_bindings: KeyBindings,
        mut mouse_bindings: MouseBindings,
    ) {
        if self.running {
            panic!("Attempt to call grab_keys_and_run while already running");
        }

        // ignore SIGCHILD and allow child / inherited processes to be inherited by pid1
        debug!("Registering SIGCHILD signal handler");
        unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) }.unwrap();

        debug!("Grabbing key and mouse bindings");
        self.conn.grab_keys(&key_bindings, &mouse_bindings);
        debug!("Forcing focus to first Workspace");
        self.focus_workspace(&Selector::Index(0));
        run_hooks!(startup, self,);
        self.running = true;

        debug!("Entering main event loop");
        while self.running {
            if let Some(event) = self.conn.wait_for_event() {
                debug!("Got XEvent: {:?}", event);
                for action in process_next_event(event, self.current_state()) {
                    if let Err(e) =
                        self.handle_event_action(action, &mut key_bindings, &mut mouse_bindings)
                    {
                        warn!("Error handling event: {}", e);
                    }
                }
                run_hooks!(event_handled, self,);
            }
            self.conn.flush();
        }
    }

    /*
     * Common state queries that we re-use in multiple places.
     */

    // If the requsted workspace index is out of bounds or not currently visible then return None.
    fn indexed_screen_for_workspace(&self, wix: usize) -> Option<(usize, &Screen)> {
        self.screens
            .indexed_element(&Selector::Condition(&|s| s.wix == wix))
    }

    // We always have at least one `Screen] so no need for an Option.
    fn active_ws_index(&self) -> usize {
        self.screens.focused().expect("there were no screens").wix
    }

    // The ordered list of currently visible [Workspace] indices (one per screen).
    fn visible_workspaces(&self) -> Vec<usize> {
        self.screens.vec_map(|s| s.wix)
    }

    // The index of the [Workspace] holding the requested X window ID. This can return None if
    // the id does not map to a [WindowManager] managed [Client] which happens if the window
    // is unmanaged (e.g. a dock or toolbar) or if a client [Hook] has requested ownership
    // of that particular [Client].
    fn workspace_index_for_client(&mut self, id: WinId) -> Option<usize> {
        self.client_map.get(&id).map(|c| c.workspace())
    }

    fn focused_client_id(&self) -> Option<WinId> {
        self.focused_client.or(self
            .workspaces
            .map_selected(&Selector::Index(self.active_ws_index()), |ws| {
                ws.focused_client()
            })?)
    }

    fn focused_client(&self) -> Option<&Client> {
        self.focused_client_id()
            .and_then(move |id| self.client_map.get(&id))
    }

    fn focused_client_mut(&mut self) -> Option<&mut Client> {
        self.focused_client_id()
            .and_then(move |id| self.client_map.get_mut(&id))
    }

    /*
     * Top Level EventAction handlers
     */

    // The given X window ID is now considered focused by the X server
    fn client_gained_focus(&mut self, id: WinId) {
        let prev_focused = self.focused_client().map(|c| c.id());
        if let Some(id) = prev_focused {
            self.client_lost_focus(id)
        }

        self.conn
            .set_client_border_color(id, self.config.focused_border);
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

    // The given X window ID lost focus according to the X server
    fn client_lost_focus(&mut self, id: WinId) {
        if self.focused_client == Some(id) {
            self.focused_client = None;
        }

        self.conn
            .set_client_border_color(id, self.config.unfocused_border);
    }

    // The given window ID has had its EWMH name updated by something
    fn client_name_changed(&mut self, id: WinId) -> Result<()> {
        let name = util::window_name(&self.conn, id)?;
        if let Some(c) = self.client_map.get_mut(&id) {
            c.set_name(&name)
        }
        run_hooks!(client_name_updated, self, id, &name, false);
        Ok(())
    }

    // The given window ID has been destroyed so remove our internal state referencing it.
    fn remove_client(&mut self, id: WinId) {
        if let Some(client) = self.client_map.remove(&id) {
            let wix = client.workspace();
            self.workspaces.apply_to(&Selector::Index(wix), |ws| {
                ws.remove_client(id);
            });

            if self.focused_client == Some(id) {
                self.focused_client = None;
            }

            if wix == self.active_ws_index() {
                self.apply_layout(self.active_ws_index());
            }

            self.update_x_known_clients();
            run_hooks!(remove_client, self, id);
        } else {
            warn!("attempt to remove unknown client {}", id);
        }
    }

    /// Query the [XConn] for the current connected [Screen] list and reposition displayed
    /// [Workspace] instances if needed.
    pub fn detect_screens(&mut self) {
        let screens = util::get_screens(
            &self.conn,
            self.visible_workspaces(),
            self.workspaces.len(),
            self.config.bar_height,
            self.config.top_bar,
        );

        if screens == self.screens.as_vec() {
            return; // nothing changed
        }

        info!("Updating known screens: {} screens detected", screens.len());
        self.screens = Ring::new(screens);
        for wix in self.visible_workspaces() {
            self.apply_layout(wix);
        }

        let regions = self.screens.vec_map(|s| s.region(false));
        run_hooks!(screens_updated, self, &regions);
    }

    // Map a new client window.
    fn handle_map_request(&mut self, id: WinId) -> Result<()> {
        let props = util::client_str_props(&self.conn, id);
        debug!(
            "Handling map request: name[{}] id[{}] class[{}] type[{}]",
            props.name, id, props.class, props.ty
        );

        if !self.conn.is_managed_window(id) {
            self.conn.map_window(id);
            return Ok(());
        }

        let classes = str_slice!(self.config.floating_classes);
        let floating = self.conn.window_should_float(id, classes);
        let mut client = Client::new(
            id,
            props.name,
            props.class,
            self.active_ws_index(),
            floating,
        );

        // Run hooks to allow them to modify the client
        run_hooks!(new_client, self, &mut client);
        let wix = client.workspace();

        if client.wm_managed {
            self.add_client_to_workspace(wix, id);
        }

        if client.floating {
            if let Some((_, s)) = self.indexed_screen_for_workspace(wix) {
                util::position_floating_client(
                    &self.conn,
                    id,
                    s.region(self.config.show_bar),
                    self.config.gap_px,
                    self.config.border_px,
                )?
            }
        }

        self.client_map.insert(id, client);
        self.conn.mark_new_window(id);
        self.client_gained_focus(id);
        self.update_x_known_clients();

        if wix == self.active_ws_index() {
            self.apply_layout(wix);
            util::map_window_if_needed(&self.conn, self.client_map.get_mut(&id));
            let s = self.screens.focused().unwrap();
            self.conn.warp_cursor(Some(id), s);
        }

        Ok(())
    }

    // NOTE: This defers control of the [WindowManager] to the user's key-binding action
    //       which can lead to arbitrary calls to public methods on the [WindowManager]
    //       including mutable methods.
    fn run_key_binding(&mut self, k: KeyCode, bindings: &mut KeyBindings) {
        debug!("handling key code: {:?}", k);
        if let Some(action) = bindings.get_mut(&k) {
            action(self); // ignoring Child handlers and SIGCHILD
        }
    }

    // NOTE: This defers control of the [WindowManager] to the user's mouse-binding action
    //       which can lead to arbitrary calls to public methods on the [WindowManager]
    //       including mutable methods.
    fn run_mouse_binding(&mut self, e: MouseEvent, bindings: &mut MouseBindings) {
        debug!("handling mouse event: {:?} {:?}", e.state, e.kind);
        if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
            action(self, &e); // ignoring Child handlers and SIGCHILD
        }
    }

    // Set the active [Screen] based on an (x, y) [Point]. If point is None then we set
    // based on the current cursor position instead.
    fn set_screen_from_point(&mut self, point: Option<Point>) {
        let point = point.unwrap_or_else(|| self.conn.cursor_position());
        self.focus_screen(&Selector::Condition(&|s: &Screen| s.contains(point)));
    }

    // Toggle the given client fullscreen. This has knock on effects for other windows and can
    // be triggered by user key bindings as well as applications requesting full screen as well.
    // TODO: should something going fullscreen also hide unmaged windows?
    fn set_fullscreen(&mut self, id: WinId, should_fullscreen: bool) -> Option<()> {
        let (currently_fullscreen, wix) = self
            .client_map
            .get_mut(&id)
            .map(|c| (c.fullscreen, c.workspace()))?;
        if currently_fullscreen == should_fullscreen {
            return None; // Client is already in the correct state, we shouldn't have been called
        }

        let r = self
            .screen(&Selector::Condition(&|s| s.wix == wix))?
            .region(false);
        let workspace = self.workspaces.get_mut(wix)?;
        if util::toggle_fullscreen(&self.conn, id, &mut self.client_map, workspace, r) {
            self.apply_layout(wix);
        }

        None
    }

    /*
     * Common mid level actions that make up larger event response handlers.
     */

    fn apply_layout(&mut self, wix: usize) {
        debug!("Attempting to layout workspace {}", wix);
        let ws = self.workspaces.get(wix).unwrap();
        let indexed_screen = self.indexed_screen_for_workspace(wix);
        if indexed_screen.is_none() {
            return; // workspace is not currently visible
        }

        let (i, s) = indexed_screen.unwrap();
        let lc = ws.layout_conf();
        if !lc.floating {
            let region = s.region(self.config.show_bar);
            let (border, gap) = (self.config.border_px, self.config.gap_px);
            util::apply_arrange_actions(
                &self.conn,
                ws.arrange(region, &self.client_map),
                &lc,
                &mut self.client_map,
                border,
                gap,
            );
        }

        run_hooks!(layout_applied, self, wix, i);
    }

    fn update_x_workspace_details(&mut self) {
        let vec_names = self.workspaces.vec_map(|w| w.name().to_string());
        let names = str_slice!(vec_names);
        self.conn.update_desktops(names);
        run_hooks!(workspaces_updated, self, names, self.active_ws_index());
    }

    fn update_x_known_clients(&self) {
        let clients: Vec<WinId> = self.client_map.keys().copied().collect();
        self.conn.update_known_clients(&clients);
    }

    fn focus_screen(&mut self, sel: &Selector<'_, Screen>) -> Option<&Screen> {
        if let Some((changed, _)) = self.screens.focus(sel) {
            if changed {
                run_hooks!(screen_change, self, self.screens.focused_index());
            }
        }
        let wix = self.screens.focused().unwrap().wix;
        self.workspaces.focus(&Selector::Index(wix));
        self.screens.focused()
    }

    fn add_client_to_workspace(&mut self, wix: usize, id: WinId) {
        let cip = self.client_insert_point;
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.add_client(id, &cip);
            self.conn.set_client_workspace(id, wix);
            run_hooks!(client_added_to_workspace, self, id, wix);
        };
    }

    /*
     * Public methods that can be triggered by user bindings or directly in the
     * user's main.rs
     */

    /// Log information out at INFO level for picking up by external programs
    pub fn log(&self, msg: &str) {
        info!("{}", msg);
    }

    /// Cycle between known [screens][Screen]. Does not wrap from first to last
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
     * Cycle between [workspaces][Workspace] on the current [screen][Screen]. This will pull
     * workspaces to the screen if they are currently displayed on another screen.
     */
    pub fn cycle_workspace(&mut self, direction: Direction) {
        self.workspaces.cycle_focus(direction);
        let i = self.workspaces.focused_index();
        self.focus_workspace(&Selector::Index(i));
    }

    /// Move the currently focused [workspace][Workspace] to the next [screen][Screen] in 'direction'
    pub fn drag_workspace(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        self.cycle_screen(direction);
        self.focus_workspace(&Selector::Index(wix)); // focus_workspace will pull it to the new screen
    }

    /// Cycle between [clients][Client] for the active [workspace][Workspace]
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

    /// Rotate the [client][Client] stack on the active [workspace][Workspace]
    pub fn rotate_clients(&mut self, direction: Direction) {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.rotate_clients(direction)
        };
    }

    /// Move the focused [client][Client] through the stack of clients on the active
    /// [workspace][Workspace]
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

    /// Cycle between [layouts][crate::core::layout::Layout] for the active [workspace][Workspace]
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

    /// Increase or decrease the current [layout][crate::core::layout::Layout] main_ratio by
    /// main_ratio_step
    pub fn update_main_ratio(&mut self, change: Change) {
        let step = self.config.main_ratio_step;
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

    /// The layout symbol for the [layout][crate::core::layout::Layout] currently being used on the
    /// active workspace
    pub fn current_layout_symbol(&self) -> &str {
        match self.workspaces.get(self.active_ws_index()) {
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
    pub fn focus_workspace(&mut self, selector: &Selector<'_, Workspace>) {
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
                ws.clients().iter().for_each(|id| {
                    util::unmap_window_if_needed(&self.conn, self.client_map.get_mut(id))
                });
            }

            if let Some(ws) = self.workspaces.get(index) {
                ws.clients().iter().for_each(|id| {
                    util::map_window_if_needed(&self.conn, self.client_map.get_mut(id))
                });
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
    pub fn client_to_workspace(&mut self, selector: &Selector<'_, Workspace>) {
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
                self.apply_layout(self.active_ws_index());

                // layout & focus the screen we just landed on if the workspace is displayed
                // otherwise unmap the window because we're no longer visible
                if self.screens.iter().any(|s| s.wix == index) {
                    self.apply_layout(index);
                    let s = self.screens.focused().unwrap();
                    self.conn.warp_cursor(Some(id), s);
                    self.focus_screen(&Selector::Index(self.active_screen_index()));
                } else {
                    util::unmap_window_if_needed(&self.conn, self.client_map.get_mut(&id))
                }
            };
        }
    }

    /// Move the focused client to the active workspace on the screen matching 'selector'.
    pub fn client_to_screen(&mut self, selector: &Selector<'_, Screen>) {
        let i = match self.screen(selector) {
            Some(s) => s.wix,
            None => return,
        };
        self.client_to_workspace(&Selector::Index(i));
    }

    /// Toggle the fullscreen state of the given client ID
    pub fn toggle_client_fullscreen(&mut self, selector: &Selector<'_, Client>) {
        let (id, client_is_fullscreen) = match self.client(selector) {
            None => return, // unknown client
            Some(c) => (c.id(), c.fullscreen),
        };
        self.set_fullscreen(id, !client_is_fullscreen);
    }

    /// Kill the focused client window.
    pub fn kill_client(&mut self) {
        let id = self.conn.focused_client();
        self.conn
            .send_client_event(id, Atom::WmDeleteWindow.as_ref())
            .unwrap();
        self.conn.flush();

        self.remove_client(id);
        self.apply_layout(self.active_ws_index());
    }

    /// Get a reference to the first Screen satisfying 'selector'. WinId selectors will return
    /// the screen containing that Client if the client is known.
    /// NOTE: It is not possible to get a mutable reference to a Screen.
    pub fn screen(&self, selector: &Selector<'_, Screen>) -> Option<&Screen> {
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
    pub fn remove_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Option<Workspace> {
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
    pub fn workspace(&self, selector: &Selector<'_, Workspace>) -> Option<&Workspace> {
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
    pub fn workspace_mut(&mut self, selector: &Selector<'_, Workspace>) -> Option<&mut Workspace> {
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
    pub fn all_workspaces(&self, selector: &Selector<'_, Workspace>) -> Vec<&Workspace> {
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
    pub fn all_workspaces_mut(
        &mut self,
        selector: &Selector<'_, Workspace>,
    ) -> Vec<&mut Workspace> {
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
    pub fn set_workspace_name(
        &mut self,
        name: impl Into<String>,
        selector: Selector<'_, Workspace>,
    ) {
        if let Some(ws) = self.workspaces.element_mut(&selector) {
            ws.set_name(name)
        };
        self.update_x_workspace_details();
    }

    /// Take a reference to the first Client found matching 'selector'
    pub fn client(&self, selector: &Selector<'_, Client>) -> Option<&Client> {
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
    pub fn client_mut(&mut self, selector: &Selector<'_, Client>) -> Option<&mut Client> {
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
    pub fn all_clients(&self, selector: &Selector<'_, Client>) -> Vec<&Client> {
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
    pub fn all_clients_mut(&mut self, selector: &Selector<'_, Client>) -> Vec<&mut Client> {
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
            .map(|s| s.region(self.config.show_bar))
    }

    /// Position an individual client on the display. (x,y) coordinates are absolute (i.e. relative
    /// to the root window not any individual screen).
    pub fn position_client(&self, id: WinId, region: Region, stack_above: bool) {
        self.conn
            .position_window(id, region, self.config.border_px, stack_above);
    }

    /// Make the Client with ID 'id' visible at its last known position.
    pub fn show_client(&mut self, id: WinId) {
        util::map_window_if_needed(&self.conn, self.client_map.get_mut(&id));
        self.conn.set_client_workspace(id, self.active_ws_index());
    }

    /// Hide the Client with ID 'id'.
    pub fn hide_client(&mut self, id: WinId) {
        util::unmap_window_if_needed(&self.conn, self.client_map.get_mut(&id));
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
    use crate::core::{data_types::*, layout::*, ring::Direction::*, screen::*, xconnection::*};

    use std::cell::Cell;

    fn wm_with_mock_conn(events: Vec<XEvent>, unmanaged_ids: Vec<WinId>) -> WindowManager {
        let conn = MockXConn::new(test_screens(), events, unmanaged_ids);
        let mut conf = Config::default();
        conf.layouts = test_layouts();
        let mut wm = WindowManager::new(conf, Box::new(conn), vec![]);
        wm.init();

        wm
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
            wm.handle_map_request(10 * (i + offset + 1) as u32).unwrap();
        }
    }

    #[test]
    fn workspace_switching_with_active_clients() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);

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
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0);
        wm.kill_client();

        assert_eq!(wm.workspaces[0].len(), 0);
    }

    #[test]
    fn kill_client_kills_focused_not_first() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
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
        let mut wm = wm_with_mock_conn(vec![], vec![]);
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
        let mut wm = wm_with_mock_conn(vec![], vec![]);
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
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(&Selector::Index(1)); // 20 -> ws::1
        wm.client_to_workspace(&Selector::Index(1)); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(&Selector::Index(1));

        assert_eq!(wm.workspaces[1].focused_client(), Some(10));
    }

    #[test]
    fn client_to_invalid_workspace_is_noop() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
        wm.client_to_workspace(&Selector::Index(42));
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
    }

    #[test]
    fn client_to_screen_sets_correct_workspace() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        wm.client_to_screen(&Selector::Index(1));
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(1));
    }

    #[test]
    fn client_to_invalid_screen_is_noop() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
        wm.client_to_screen(&Selector::Index(5));
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
    }

    #[test]
    fn x_focus_events_set_workspace_focus() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 5, 0); // focus on last client: 50
        wm.client_gained_focus(10);

        assert_eq!(wm.workspaces[0].focused_client(), Some(10));
    }

    #[test]
    fn focus_workspace_sets_focus_in_ring() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        assert_eq!(wm.workspaces.focused_index(), 0);
        assert_eq!(wm.workspaces.focused_index(), wm.active_ws_index());
        wm.focus_workspace(&Selector::Index(3));
        assert_eq!(wm.workspaces.focused_index(), 3);
        assert_eq!(wm.workspaces.focused_index(), wm.active_ws_index());
    }

    #[test]
    fn dragging_clients_forward_from_index_0() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
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
        let mut wm = wm_with_mock_conn(vec![], vec![]);

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
        let mut wm = wm_with_mock_conn(vec![], vec![]);

        add_n_clients(&mut wm, 3, 0);
        wm.focus_workspace(&Selector::Index(1));
        add_n_clients(&mut wm, 2, 3);

        assert_eq!(wm.all_workspaces(&Selector::WinId(40))[0].name(), "2");
        assert_eq!(wm.all_workspaces_mut(&Selector::WinId(10))[0].name(), "1");
    }

    #[test]
    fn selector_screen() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
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
        let mut wm = wm_with_mock_conn(vec![], vec![]);
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
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 4, 0);

        assert_eq!(wm.client(&Selector::Focused), wm.client_map.get(&40));
        assert_eq!(wm.client(&Selector::Index(2)), wm.client_map.get(&20));
        assert_eq!(wm.client(&Selector::WinId(30)), wm.client_map.get(&30));
        assert_eq!(
            wm.client(&Selector::Condition(&|c| c.id() == 10)),
            wm.client_map.get(&10)
        );
    }

    #[test]
    fn unmanaged_window_types_are_not_tracked() {
        // Setting the unmanaged window IDs here sets the return of
        // MockXConn.is_managed_window to false for those IDs
        let mut wm = wm_with_mock_conn(vec![], vec![10]);

        wm.handle_map_request(10).unwrap(); // should not be tiled
        assert!(wm.client_map.get(&10).is_none());
        assert!(wm.workspaces[0].is_empty());

        wm.handle_map_request(20).unwrap(); // should be tiled
        assert!(wm.client_map.get(&20).is_some());
        assert!(wm.workspaces[0].len() == 1);
    }

    struct ScreenChangingXConn {
        num_screens: Cell<usize>,
    }
    impl StubXConn for ScreenChangingXConn {
        fn mock_current_outputs(&self) -> Vec<Screen> {
            let num_screens = self.num_screens.get();
            let screens = (0..(num_screens))
                .map(|n| Screen::new(Region::new(800 * n as u32, 600 * n as u32, 800, 600), n))
                .collect();
            self.num_screens.set(num_screens + 1);
            screens
        }

        // Hack to reset the screen count without needing RefCell
        fn mock_set_root_window_name(&self, _: &str) {
            self.num_screens.set(1);
        }
    }

    #[test]
    fn updating_screens_retains_focused_workspaces() {
        let conn = ScreenChangingXConn {
            num_screens: Cell::new(1),
        };
        let conf = Config::default();
        let mut wm = WindowManager::new(conf, Box::new(conn), vec![]);
        wm.init();

        // detect_screens is called on init so should have one screen
        assert_eq!(wm.screens.len(), 1);
        assert_eq!(wm.screens.focused().unwrap().wix, 0);

        // Focus workspace 1 the redetect screens: should have 1 and 0
        wm.focus_workspace(&Selector::Index(1));
        assert_eq!(wm.screens.focused().unwrap().wix, 1);
        wm.detect_screens(); // adds a screen due to ScreenChangingXConn impl
        assert_eq!(wm.screens.len(), 2);
        assert_eq!(wm.screens.get(0).unwrap().wix, 1);
        assert_eq!(wm.screens.get(1).unwrap().wix, 0);

        // Adding another screen should now have WS 2 as 1 is taken
        wm.detect_screens(); // adds a screen due to ScreenChangingXConn impl
        assert_eq!(wm.screens.len(), 3);
        assert_eq!(wm.screens.get(0).unwrap().wix, 1);
        assert_eq!(wm.screens.get(1).unwrap().wix, 0);
        assert_eq!(wm.screens.get(2).unwrap().wix, 2);

        // Focus WS 3 on screen 1, drop down to 1 screen: it should still have WS 3
        wm.focus_workspace(&Selector::Index(3));
        wm.conn.set_root_window_name("reset the screen count to 1");
        wm.detect_screens(); // Should now have one screen
        assert_eq!(wm.screens.len(), 1);
        assert_eq!(wm.screens.get(0).unwrap().wix, 3);
    }
}
