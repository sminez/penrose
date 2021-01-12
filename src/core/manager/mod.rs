//! The main user API and control logic for Penrose.
use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
        client::Client,
        config::Config,
        data_types::{Change, Point, Region, WinId},
        hooks::Hooks,
        ring::{Direction, InsertPoint, Ring, Selector},
        screen::Screen,
        workspace::Workspace,
        xconnection::{Atom, XConn},
    },
    ErrorHandler, PenroseError, Result,
};

#[cfg(feature = "serde")]
use crate::core::{helpers::logging_error_handler, layout::LayoutFunc};

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
        let res = hooks.iter_mut().try_for_each(|h| h.$method($_self, $($arg),*));
        $_self.hooks.replace(hooks);
        if let Err(e) = res {
            ($_self.error_handler)(e);
        }
    };
}

#[cfg(feature = "serde")]
fn default_hooks<X: XConn>() -> Cell<Hooks<X>> {
    Cell::new(Vec::new())
}

/// WindowManager is the primary struct / owner of the event loop for penrose.
///
/// It handles most (if not all) of the communication with the underlying [XConn], responding to
/// [XEvent][crate::core::xconnection::XEvent]s emitted by it. User key / mouse bindings are parsed
/// and bound on the call to `grab_keys_and_run` and then triggered when corresponding `XEvent`
/// instances come through in the main event loop.
///
/// # A note on examples
///
/// The examples provided for each of the `WindowManager` methods are written using an example
/// implementation of [XConn] that mocks out calls to the X server. In each case, it is assumed
/// that you have an initialised `WindowManager` struct as demonstrated in the full examples for
/// `new` and `init`.
///
/// For full examples of how to configure the `WindowManager`, please see the [examples][1]
/// directory in the Penrose repo.
///
/// [1]: https://github.com/sminez/penrose/tree/develop/examples
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct WindowManager<X: XConn> {
    conn: X,
    config: Config,
    screens: Ring<Screen>,
    workspaces: Ring<Workspace>,
    client_map: HashMap<WinId, Client>,
    #[cfg_attr(feature = "serde", serde(skip, default = "default_hooks"))]
    hooks: Cell<Hooks<X>>,
    previous_workspace: usize,
    client_insert_point: InsertPoint,
    focused_client: Option<WinId>,
    running: bool,
    #[cfg_attr(feature = "serde", serde(skip, default = "logging_error_handler"))]
    error_handler: ErrorHandler,
    #[cfg_attr(feature = "serde", serde(skip))]
    hydrated: bool,
}

impl<X: XConn> fmt::Debug for WindowManager<X> {
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

impl<X: XConn> WindowManager<X> {
    /// Construct a new window manager instance using a chosen [XConn] backed to communicate
    /// with the X server.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use penrose::{
    ///     core::{Config, WindowManager},
    ///     xcb::XcbConnection,
    ///     logging_error_handler
    /// };
    ///
    /// let mut wm = WindowManager::new(
    ///     Config::default(),
    ///     XcbConnection::new().unwrap(),
    ///     vec![],
    ///     logging_error_handler(),
    /// );
    ///
    /// if let Err(e) = wm.init() {
    ///     panic!("failed to initialise WindowManager: {}", e);
    /// }
    ///
    /// wm.log("ready to call grab_keys_and_run!").unwrap();
    /// ```
    pub fn new(config: Config, conn: X, hooks: Hooks<X>, error_handler: ErrorHandler) -> Self {
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
            hydrated: true,
            error_handler,
        }
    }

    /// Restore missing state following serde deserialization.
    ///
    /// # Errors
    /// The deserialized state will be checked and validated for internal consistency
    /// and consistency with the current X server state using the deserialized [XConn].
    /// If the state is not a valid snapshot then an error will be returned. Examples of invalid
    /// state include:
    ///   - Not providing a required layout function in `layout_funcs`
    ///   - [Workspace] [Client] IDs not appearing in the [WindowManager] client_map
    ///   - Being unable to connect to the X Server
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::{Result, map};
    /// use penrose::{
    ///     contrib::hooks::{SpawnRule, ClientSpawnRules},
    ///     core::{
    ///         hooks::Hooks,
    ///         layout::{floating, side_stack, LayoutFunc},
    ///         manager::WindowManager,
    ///     },
    ///     xcb::XcbConnection,
    ///     logging_error_handler
    /// };
    ///
    /// # fn example() -> Result<()> {
    /// // Hooks that we want to set up on restart
    /// let hooks: Hooks<_> = vec![
    ///     ClientSpawnRules::new(vec![
    ///         SpawnRule::ClassName("xterm-256color" , 3),
    ///         SpawnRule::WMName("Firefox Developer Edition" , 7),
    ///     ])
    /// ];
    ///
    /// // The layout functions we were using previously
    /// let layout_funcs = map! {
    ///     "[side]" => side_stack as LayoutFunc,
    ///     "[----]" => floating as LayoutFunc,
    /// };
    ///
    /// let json_str = "...";  // Load in the serialized state from somewhere
    /// let mut manager: WindowManager<XcbConnection> = serde_json::from_str(&json_str).unwrap();
    /// assert!(manager.hydrate_and_init(hooks, logging_error_handler(), layout_funcs).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "serde")]
    pub fn hydrate_and_init(
        &mut self,
        hooks: Hooks<X>,
        error_handler: ErrorHandler,
        layout_funcs: HashMap<&str, LayoutFunc>,
    ) -> Result<()> {
        self.conn.hydrate()?;
        self.hooks.set(hooks);
        self.error_handler = error_handler;
        self.workspaces
            .iter_mut()
            .try_for_each(|w| w.restore_layout_functions(&layout_funcs))?;

        util::validate_hydrated_wm_state(self)?;
        self.hydrated = true;
        self.init()?;
        Ok(())
    }

    /// This initialises the [WindowManager] internal state but does not start processing any
    /// events from the X server. If you need to perform any custom setup logic with the
    /// [WindowManager] itself, it should be run after calling this method and before
    /// [WindowManager::grab_keys_and_run].
    ///
    /// # Example
    ///
    /// See [new][WindowManager::new]
    pub fn init(&mut self) -> Result<()> {
        if !self.hydrated {
            panic!("Need to call 'hydrate_and_init' when restoring from serialised state")
        }

        debug!("Attempting initial screen detection");
        self.detect_screens()?;

        debug!("Setting EWMH properties");
        self.conn
            .set_wm_properties(str_slice!(self.config.workspaces));

        debug!("Forcing cursor to first screen");
        self.conn.warp_cursor(None, &self.screens[0]);

        Ok(())
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
        key_bindings: &mut KeyBindings<X>,
        mouse_bindings: &mut MouseBindings<X>,
    ) -> Result<()> {
        debug!("Handling event action: {:?}", action);
        match action {
            EventAction::ClientFocusGained(id) => self.client_gained_focus(id),
            EventAction::ClientFocusLost(id) => self.client_lost_focus(id),
            EventAction::ClientNameChanged(id, is_root) => self.client_name_changed(id, is_root)?,
            EventAction::DestroyClient(id) => self.remove_client(id),
            EventAction::DetectScreens => {
                run_hooks!(randr_notify, self,);
                self.detect_screens()?
            }
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

    /// This is the main event loop for the [WindowManager].
    ///
    /// The `XConn` [wait_for_event][1] method is called to fetch the next event from the X server,
    /// after which it is processed into a set of internal EventActions which are then processed by
    /// the [WindowManager] to update state and perform actions. This method is an infinite loop
    /// until the [exit][2] method is called, which triggers the `XConn` [cleanup][3] before
    /// exiting the loop. You can provide any additional teardown logic you need your main.rs after
    /// the call to `grab_keys_and_run` and all internal state will still be accessible, though
    /// methods requiring the use of the [XConn] will fail.
    ///
    /// [1]: crate::core::xconnection::XConn::wait_for_event
    /// [2]: WindowManager::exit
    /// [3]: crate::core::xconnection::XConn::cleanup
    pub fn grab_keys_and_run(
        &mut self,
        mut key_bindings: KeyBindings<X>,
        mut mouse_bindings: MouseBindings<X>,
    ) -> Result<()> {
        if self.running {
            panic!("Attempt to call grab_keys_and_run while already running");
        }
        if !self.hydrated {
            panic!("'hydrate_and_init' must be called before 'grab_keys_and_run' when restoring from serialised state")
        }

        // ignore SIGCHILD and allow child / inherited processes to be inherited by pid1
        debug!("Registering SIGCHILD signal handler");
        if let Err(e) = unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) } {
            panic!("unable to set signal handler: {}", e);
        }

        debug!("Grabbing key and mouse bindings");
        self.conn.grab_keys(&key_bindings, &mouse_bindings);
        debug!("Forcing focus to first Workspace");
        self.focus_workspace(&Selector::Index(0))?;
        run_hooks!(startup, self,);
        self.running = true;

        debug!("Entering main event loop");
        while self.running {
            match self.conn.wait_for_event() {
                Ok(event) => {
                    debug!("Got XEvent: {:?}", event);
                    for action in process_next_event(event, self.current_state()) {
                        if let Err(e) =
                            self.handle_event_action(action, &mut key_bindings, &mut mouse_bindings)
                        {
                            (self.error_handler)(e);
                        }
                    }
                    run_hooks!(event_handled, self,);
                    self.conn.flush();
                }

                Err(e) => {
                    self.exit()?;
                    return Err(e);
                }
            }
        }

        Ok(())
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

    /// The [WinId] of the client that currently has focus.
    ///
    /// Returns `None` if there are no clients to focus.
    pub fn focused_client_id(&self) -> Option<WinId> {
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
                let prev_was_in_ws = prev_focused.map_or(false, |id| ws.client_ids().contains(&id));
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
    fn client_name_changed(&mut self, id: WinId, is_root: bool) -> Result<()> {
        let name = util::window_name(&self.conn, id)?;
        if !is_root {
            if let Some(c) = self.client_map.get_mut(&id) {
                c.set_name(&name)
            }
        }
        run_hooks!(client_name_updated, self, id, &name, is_root);
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
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// assert_eq!(manager.n_screens(), 1);
    ///
    /// // Simulate a monitor being attached
    /// manager.conn_mut().set_screen_count(2);
    ///
    /// manager.detect_screens()?;
    /// assert_eq!(manager.n_screens(), 2);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn detect_screens(&mut self) -> Result<()> {
        let screens = util::get_screens(
            &self.conn,
            self.visible_workspaces(),
            self.workspaces.len(),
            self.config.bar_height,
            self.config.top_bar,
        );

        if screens == self.screens.as_vec() {
            return Ok(()); // nothing changed
        }

        info!("Updating known screens: {} screens detected", screens.len());
        self.screens = Ring::new(screens);
        for wix in self.visible_workspaces() {
            self.apply_layout(wix);
        }

        let regions = self.screens.vec_map(|s| s.region(false));
        run_hooks!(screens_updated, self, &regions);

        Ok(())
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
            let s = self.screens.focused_unchecked();
            self.conn.warp_cursor(Some(id), s);
        }

        Ok(())
    }

    // NOTE: This defers control of the [WindowManager] to the user's key-binding action
    //       which can lead to arbitrary calls to public methods on the [WindowManager]
    //       including mutable methods.
    fn run_key_binding(&mut self, k: KeyCode, bindings: &mut KeyBindings<X>) {
        debug!("handling key code: {:?}", k);
        if let Some(action) = bindings.get_mut(&k) {
            // ignoring Child handlers and SIGCHILD
            if let Err(e) = action(self) {
                (self.error_handler)(e);
            }
        }
    }

    // NOTE: This defers control of the [WindowManager] to the user's mouse-binding action
    //       which can lead to arbitrary calls to public methods on the [WindowManager]
    //       including mutable methods.
    fn run_mouse_binding(&mut self, e: MouseEvent, bindings: &mut MouseBindings<X>) {
        debug!("handling mouse event: {:?} {:?}", e.state, e.kind);
        if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
            // ignoring Child handlers and SIGCHILD
            if let Err(e) = action(self, &e) {
                (self.error_handler)(e);
            }
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
        let ws = match self.workspaces.get(wix) {
            Some(ws) => ws,
            None => {
                error!("attempt to layout unknown workspace: {}", wix);
                return;
            }
        };

        let (i, s) = match self.indexed_screen_for_workspace(wix) {
            Some(index_and_screen) => index_and_screen,
            None => return, // workspace is not currently visible
        };

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
        let wix = self.screens.focused_unchecked().wix;
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

    /// Get an immutable reference to the underlying [XConn] impl that backs this [WindowManager]
    ///
    /// # A word of warning
    ///
    /// This method is provided as a utility for allowing you to make use of implementation
    /// specific methods on the `XConn` impl that your `WindowManager` is using. You will need to
    /// take care not to manipulate X state via this as you may end up with inconsistant state in
    /// the `WindowManager`.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// // a helper method on the ExampleXConn used for these examples
    /// assert_eq!(manager.conn().current_screen_count(), 1);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn conn(&self) -> &X {
        &self.conn
    }

    /// Get an mutable reference to the underlying [XConn] impl that backs this [WindowManager]
    ///
    /// # A word of warning
    ///
    /// This method is provided as a utility for allowing you to make use of implementation
    /// specific methods on the `XConn` impl that your `WindowManager` is using. You will need to
    /// take care not to manipulate X state via this as you may end up with inconsistant state in
    /// the `WindowManager`.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// // a helper method on the ExampleXConn used for these examples
    /// assert_eq!(manager.conn().current_screen_count(), 1);
    ///
    /// manager.conn_mut().set_screen_count(2);
    /// assert_eq!(manager.conn().current_screen_count(), 2);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn conn_mut(&mut self) -> &mut X {
        &mut self.conn
    }

    /// Log information out at INFO level for picking up by external programs
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// manager.log("hello from penrose!")?;
    /// manager.log(format!("This manager has {} screens", manager.n_screens()))?;
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn log(&self, msg: impl Into<String>) -> Result<()> {
        info!("{}", msg.into());

        Ok(())
    }

    /// Cycle between known [screens][Screen]. Does not wrap from first to last
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// // manager here is an example window manager with two screens
    /// assert_eq!(manager.active_screen_index(), 0);
    ///
    /// manager.cycle_screen(Forward)?;
    /// assert_eq!(manager.active_screen_index(), 1);
    ///
    /// // no wrapping
    /// manager.cycle_screen(Forward)?;
    /// assert_eq!(manager.active_screen_index(), 1);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(2, vec![])).unwrap();
    /// ```
    pub fn cycle_screen(&mut self, direction: Direction) -> Result<()> {
        if !self.screens.would_wrap(direction) {
            self.screens.cycle_focus(direction);
            let i = self.screens.focused_unchecked().wix;
            self.workspaces.focus(&Selector::Index(i));
            self.conn
                .warp_cursor(None, self.screens.focused_unchecked());
            let wix = self.workspaces.focused_index();
            self.conn.set_current_workspace(wix);

            let i = self.screens.focused_index();
            run_hooks!(screen_change, self, i);
        }

        Ok(())
    }

    /// Cycle between [workspaces][1] on the current [screen][2].
    ///
    /// This method will pull workspaces to the active screen if they are currently displayed on
    /// another screen.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// // manager here is using the default Config with 9 workspaces
    ///
    /// assert_eq!(manager.focused_workspaces(), vec![0]);
    ///
    /// manager.cycle_workspace(Forward)?;
    /// assert_eq!(manager.focused_workspaces(), vec![1]);
    ///
    /// manager.cycle_workspace(Backward)?;
    /// manager.cycle_workspace(Backward)?;
    /// assert_eq!(manager.focused_workspaces(), vec![8]);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    ///
    /// [1]: Workspace
    /// [2]: Screen
    pub fn cycle_workspace(&mut self, direction: Direction) -> Result<()> {
        self.workspaces.cycle_focus(direction);
        let i = self.workspaces.focused_index();
        self.focus_workspace(&Selector::Index(i))
    }

    /// Move the currently focused [Workspace] to the next [Screen] in 'direction'
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// assert_eq!(manager.focused_workspaces(), vec![0, 1]);
    ///
    /// manager.drag_workspace(Forward)?;
    /// assert_eq!(manager.focused_workspaces(), vec![1, 0]);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(2, vec![])).unwrap();
    /// ```
    pub fn drag_workspace(&mut self, direction: Direction) -> Result<()> {
        let wix = self.active_ws_index();
        self.cycle_screen(direction)?;
        self.focus_workspace(&Selector::Index(wix)) // focus_workspace will pull it to the new screen
    }

    /// Cycle focus between [clients][1] for the active [Workspace]
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// assert_eq!(manager.focused_client_id(), Some(0));
    ///
    /// manager.cycle_client(Backward)?;
    /// assert_eq!(manager.focused_client_id(), Some(1));
    ///
    /// manager.cycle_client(Backward)?;
    /// assert_eq!(manager.focused_client_id(), Some(2));
    ///
    /// manager.cycle_client(Backward)?;
    /// assert_eq!(manager.focused_client_id(), Some(0));
    /// # Ok(())
    /// # }
    /// #
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), HashMap::new()).unwrap();
    /// # manager.focus_client(&Selector::WinId(0)).unwrap();
    /// # example(manager).unwrap();
    /// ```
    ///
    /// [1]: Client
    pub fn cycle_client(&mut self, direction: Direction) -> Result<()> {
        let wix = self.active_ws_index();
        let res = self
            .workspaces
            .get_mut(wix)
            .and_then(|ws| ws.cycle_client(direction));
        if let Some((prev, new)) = res {
            self.client_lost_focus(prev);
            self.client_gained_focus(new);
            let screen = self.screens.focused_unchecked();
            self.conn.warp_cursor(Some(new), screen);
        }

        Ok(())
    }

    /// Focus the [Client] matching the given [Selector]
    ///
    /// # Errors
    ///
    /// If the selector matches a known client then that client is focused and `Ok(id)`
    /// is returned. If the selector doesn't match (either it was invalid or there is
    /// no focused client) then `Err(self.focused_client_id())` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// let focused = manager.focus_client(&Selector::WinId(0));
    /// assert_eq!(focused, Ok(0));
    ///
    /// let focused = manager.focus_client(&Selector::WinId(42));
    /// assert_eq!(focused, Err(Some(0))); // the current focused client
    ///
    /// let focused = manager.focus_client(&Selector::WinId(1));
    /// assert_eq!(focused, Ok(1));
    ///
    /// let focused = manager.focus_client(&Selector::WinId(42));
    /// assert_eq!(focused, Err(Some(1))); // the current focused client
    /// # Ok(())
    /// # }
    /// #
    /// # fn example2(mut manager: ExampleWM) -> Result<()> {
    ///
    /// // Or, if there are no clients to focus
    /// let focused = manager.focus_client(&Selector::WinId(0));
    /// assert_eq!(focused, Err(None));
    /// # Ok(())
    /// # }
    /// #
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), HashMap::new()).unwrap();
    /// # example(manager).unwrap();
    /// # example2(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn focus_client(
        &mut self,
        selector: &Selector<'_, Client>,
    ) -> std::result::Result<WinId, Option<WinId>> {
        let id = match self.client(selector) {
            Some(c) => c.id(),
            None => return Err(self.focused_client_id()),
        };
        self.client_gained_focus(id);
        let screen = self.screens.focused_unchecked();
        self.conn.warp_cursor(Some(id), screen);
        Ok(id)
    }

    /// Rotate the [Client] stack on the active [Workspace].
    ///
    /// This maintains the current window layout but permutes the positions of each window within
    /// that layout.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    ///
    /// manager.rotate_clients(Forward)?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![0, 2, 1]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn rotate_clients(&mut self, direction: Direction) -> Result<()> {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.rotate_clients(direction)
        };

        Ok(())
    }

    /// Move the focused [Client] through the stack of clients on the active [Workspace].
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// assert_eq!(manager.active_workspace().client_ids(), vec![3, 2, 1, 0]);
    ///
    /// manager.drag_client(Forward)?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 3, 1, 0]);
    ///
    /// manager.drag_client(Forward)?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 3, 0]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(4));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn drag_client(&mut self, direction: Direction) -> Result<()> {
        if let Some(id) = self.focused_client().map(|c| c.id()) {
            let wix = self.active_ws_index();
            self.workspaces
                .get_mut(wix)
                .and_then(|ws| ws.drag_client(direction));
            self.apply_layout(wix);
            self.client_gained_focus(id);
            self.conn
                .warp_cursor(Some(id), self.screens.focused_unchecked());
        }

        Ok(())
    }

    /// Cycle between [layouts][1] for the active [Workspace]
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> Result<()> {
    /// assert_eq!(manager.active_workspace().layout_symbol(), "first");
    ///
    /// manager.cycle_layout(Forward)?;
    /// assert_eq!(manager.active_workspace().layout_symbol(), "second");
    ///
    /// // Wrap at the end of the layout list
    /// manager.cycle_layout(Forward)?;
    /// assert_eq!(manager.active_workspace().layout_symbol(), "first");
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    ///
    /// [1]: crate::core::layout::Layout
    pub fn cycle_layout(&mut self, direction: Direction) -> Result<()> {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.cycle_layout(direction);
        }
        run_hooks!(layout_change, self, wix, self.active_screen_index());
        self.apply_layout(wix);

        Ok(())
    }

    /// Increase or decrease the number of clients in the main area by 1.
    ///
    /// The change is applied to the active [layout][1] on the [Workspace] that currently holds
    /// focus.
    ///
    /// [1]: crate::core::layout::Layout
    pub fn update_max_main(&mut self, change: Change) -> Result<()> {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.update_max_main(change)
        };
        self.apply_layout(wix);

        Ok(())
    }

    /// Increase or decrease the current [layout][crate::core::layout::Layout] main_ratio by
    /// `main_ratio_step`
    ///
    /// The change is applied to the active [layout][1] on the [Workspace] that currently holds
    /// focus.
    ///
    /// [1]: crate::core::layout::Layout
    pub fn update_main_ratio(&mut self, change: Change) -> Result<()> {
        let step = self.config.main_ratio_step;
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.update_main_ratio(change, step)
        }
        self.apply_layout(wix);

        Ok(())
    }

    /// Shut down the WindowManager, running any required cleanup and exiting penrose
    pub fn exit(&mut self) -> Result<()> {
        self.conn.cleanup();
        self.conn.flush();
        self.running = false;

        Ok(())
    }

    /// The layout symbol for the [layout][1] currently being used on the
    /// active workspace
    ///
    /// [1]: crate::core::layout::Layout
    pub fn current_layout_symbol(&self) -> &str {
        match self.workspaces.get(self.active_ws_index()) {
            Some(ws) => ws.layout_symbol(),
            None => "???",
        }
    }

    /// Set the root X window name. Useful for exposing information to external programs
    pub fn set_root_window_name(&self, s: &str) -> Result<()> {
        self.conn.set_root_window_name(s);

        Ok(())
    }

    /// Set the insert point for new clients. Default is to insert at index 0.
    pub fn set_client_insert_point(&mut self, cip: InsertPoint) -> Result<()> {
        self.client_insert_point = cip;

        Ok(())
    }

    /// Set the displayed workspace for the focused screen to be `index` in the list of
    /// workspaces passed at `init`.
    pub fn focus_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Result<()> {
        let active_ws = Selector::Index(self.screens.focused_unchecked().wix);
        if self.workspaces.equivalent_selectors(selector, &active_ws) {
            return Ok(());
        }

        if let Some(index) = self.workspaces.index(selector) {
            let active = self.active_ws_index();
            self.previous_workspace = active;

            for i in 0..self.screens.len() {
                if self.screens[i].wix == index {
                    // The workspace we want is currently displayed on another screen so
                    // pull the target workspace to the focused screen, and place the
                    // workspace we had on the screen where the target was
                    self.screens[i].wix = self.screens.focused_unchecked().wix;
                    self.screens.focused_mut_unchecked().wix = index;

                    // re-apply layouts as screen dimensions may differ
                    self.apply_layout(active);
                    self.apply_layout(index);

                    let ws = self.workspaces.get(index);
                    if let Some(id) = ws.and_then(|ws| ws.focused_client()) {
                        self.client_gained_focus(id)
                    };

                    self.workspaces.focus(&Selector::Index(index));
                    run_hooks!(workspace_change, self, active, index);
                    return Ok(());
                }
            }

            // target not currently displayed so unmap what we currently have
            // displayed and replace it with the target workspace
            if let Some(ws) = self.workspaces.get(active) {
                ws.client_ids().iter().for_each(|id| {
                    util::unmap_window_if_needed(&self.conn, self.client_map.get_mut(id))
                });
            }

            if let Some(ws) = self.workspaces.get(index) {
                ws.client_ids().iter().for_each(|id| {
                    util::map_window_if_needed(&self.conn, self.client_map.get_mut(id))
                });
            }

            self.screens.focused_mut_unchecked().wix = index;
            self.apply_layout(index);
            self.conn.set_current_workspace(index);

            let ws = self.workspaces.get(index);
            if let Some(id) = ws.and_then(|ws| ws.focused_client()) {
                self.client_gained_focus(id)
            };

            self.workspaces.focus(&Selector::Index(index));
            run_hooks!(workspace_change, self, active, index);
        }

        Ok(())
    }

    /// Switch focus back to the last workspace that had focus.
    pub fn toggle_workspace(&mut self) -> Result<()> {
        self.focus_workspace(&Selector::Index(self.previous_workspace))
    }

    /// Move the focused client to the workspace matching 'selector'.
    pub fn client_to_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Result<()> {
        let active_ws = Selector::Index(self.screens.focused_unchecked().wix);
        if self.workspaces.equivalent_selectors(&selector, &active_ws) {
            return Ok(());
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
                    let s = self.screens.focused_unchecked();
                    self.conn.warp_cursor(Some(id), s);
                    self.focus_screen(&Selector::Index(self.active_screen_index()));
                } else {
                    util::unmap_window_if_needed(&self.conn, self.client_map.get_mut(&id))
                }
            };
        }

        Ok(())
    }

    /// Move the focused client to the active workspace on the screen matching 'selector'.
    pub fn client_to_screen(&mut self, selector: &Selector<'_, Screen>) -> Result<()> {
        let i = match self.screen(selector) {
            Some(s) => s.wix,
            None => return Ok(()),
        };
        self.client_to_workspace(&Selector::Index(i))
    }

    /// Toggle the fullscreen state of the given client ID
    pub fn toggle_client_fullscreen(&mut self, selector: &Selector<'_, Client>) -> Result<()> {
        let (id, client_is_fullscreen) = match self.client(selector) {
            None => return Ok(()), // unknown client
            Some(c) => (c.id(), c.fullscreen),
        };
        self.set_fullscreen(id, !client_is_fullscreen);

        Ok(())
    }

    /// Kill the focused client window.
    pub fn kill_client(&mut self) -> Result<()> {
        let id = self.conn.focused_client();
        let del = Atom::WmDeleteWindow.as_ref();
        if let Err(e) = self.conn.send_client_event(id, del) {
            error!("Error killing client: {}", e);
        }
        self.conn.flush();

        self.remove_client(id);
        self.apply_layout(self.active_ws_index());

        Ok(())
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

    /// An immutable reference to the current active [Workspace]
    pub fn active_workspace(&self) -> &Workspace {
        self.workspaces
            .element(&Selector::Index(self.active_ws_index()))
            .unwrap()
    }

    /// A mutable reference to the current active [Workspace]
    pub fn active_workspace_mut(&mut self) -> &Workspace {
        self.workspaces
            .element_mut(&Selector::Index(self.active_ws_index()))
            .unwrap()
    }

    /// The currently focused workspace indices being shown on each screen
    pub fn focused_workspaces(&self) -> Vec<usize> {
        self.screens.iter().map(|s| s.wix).collect()
    }

    /// Add a new workspace at `index`, shifting all workspaces with indices greater to the right.
    pub fn add_workspace(&mut self, index: usize, ws: Workspace) -> Result<()> {
        self.workspaces.insert(index, ws);
        self.update_x_workspace_details();

        Ok(())
    }

    /// Add a new workspace at the end of the current workspace list
    pub fn push_workspace(&mut self, ws: Workspace) -> Result<()> {
        self.workspaces.push(ws);
        self.update_x_workspace_details();

        Ok(())
    }

    /// Remove a Workspace from the WindowManager. All clients that were present on the removed
    /// workspace will be destroyed. WinId selectors will be ignored.
    pub fn remove_workspace(
        &mut self,
        selector: &Selector<'_, Workspace>,
    ) -> Result<Option<Workspace>> {
        if self.workspaces.len() == self.screens.len() {
            return Err(PenroseError::Raw(
                "must have at least one workspace per screen".into(),
            ));
        }

        let ws = self
            .workspaces
            .remove(&selector)
            .ok_or_else(|| PenroseError::Raw("unknown workspace".into()))?;
        ws.iter().for_each(|c| self.remove_client(*c));

        // Focus the workspace before the one we just removed. There is always at least one
        // workspace before this one due to the guard above.
        let ix = self.screens.focused_unchecked().wix - 1;
        self.focus_workspace(&Selector::Index(ix))?;

        self.update_x_workspace_details();
        Ok(Some(ws))
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
    ) -> Result<()> {
        if let Some(ws) = self.workspaces.element_mut(&selector) {
            ws.set_name(name)
        };
        self.update_x_workspace_details();

        Ok(())
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
    pub fn position_client(&self, id: WinId, region: Region, stack_above: bool) -> Result<()> {
        self.conn
            .position_window(id, region, self.config.border_px, stack_above);
        Ok(())
    }

    /// Make the Client with ID 'id' visible at its last known position.
    pub fn show_client(&mut self, id: WinId) -> Result<()> {
        util::map_window_if_needed(&self.conn, self.client_map.get_mut(&id));
        self.conn.set_client_workspace(id, self.active_ws_index());
        Ok(())
    }

    /// Hide the Client with ID 'id'.
    pub fn hide_client(&mut self, id: WinId) -> Result<()> {
        util::unmap_window_if_needed(&self.conn, self.client_map.get_mut(&id));
        Ok(())
    }

    /// Layout the workspace currently shown on the given screen index.
    pub fn layout_screen(&mut self, screen_index: usize) -> Result<()> {
        if let Some(wix) = self.screens.get(screen_index).map(|s| s.wix) {
            self.apply_layout(wix)
        }

        Ok(())
    }

    /// An index into the WindowManager known screens for the screen that is currently focused
    pub fn active_screen_index(&self) -> usize {
        self.screens.focused_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        data_types::*, helpers::logging_error_handler, layout::*, ring::Direction::*, screen::*,
        xconnection::*,
    };

    use std::cell::Cell;

    fn wm_with_mock_conn(
        events: Vec<XEvent>,
        unmanaged_ids: Vec<WinId>,
    ) -> WindowManager<MockXConn> {
        let conn = MockXConn::new(test_screens(), events, unmanaged_ids);
        let conf = Config {
            layouts: test_layouts(),
            ..Default::default()
        };
        let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
        wm.init().unwrap();

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

    fn add_n_clients<X: XConn>(wm: &mut WindowManager<X>, n: usize, offset: usize) {
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
        wm.focus_workspace(&Selector::Index(1)).unwrap();
        add_n_clients(&mut wm, 2, 3);
        assert_eq!(wm.workspaces[1].len(), 2);
        assert_eq!(wm.workspaces[1].focused_client(), Some(50));

        // switch back: clients should be the same, same client should have focus
        wm.focus_workspace(&Selector::Index(0)).unwrap();
        assert_eq!(wm.workspaces[0].len(), 3);
        assert_eq!(wm.workspaces[0].focused_client(), Some(30));
    }

    #[test]
    fn killing_a_client_removes_it_from_the_workspace() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0);
        wm.kill_client().unwrap();

        assert_eq!(wm.workspaces[0].len(), 0);
    }

    #[test]
    fn kill_client_kills_focused_not_first() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 5, 0); // 50 40 30 20 10, 50 focused
        assert_eq!(wm.active_ws_index(), 0);
        wm.cycle_client(Forward).unwrap(); // 40 focused
        assert_eq!(wm.workspaces[0].focused_client(), Some(40));
        wm.kill_client().unwrap(); // remove 40, focus 30

        let ids: Vec<WinId> = wm.workspaces[0].iter().cloned().collect();
        assert_eq!(ids, vec![50, 30, 20, 10]);
        assert_eq!(wm.workspaces[0].focused_client(), Some(30));
    }

    #[test]
    fn moving_then_deleting_clients() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 2, 0);
        wm.client_to_workspace(&Selector::Index(1)).unwrap();
        wm.client_to_workspace(&Selector::Index(1)).unwrap();
        wm.focus_workspace(&Selector::Index(1)).unwrap();
        wm.kill_client().unwrap();

        // should have removed first client on ws::1 (last sent from ws::0)
        assert_eq!(wm.workspaces[1].iter().collect::<Vec<&WinId>>(), vec![&20]);
    }

    #[test]
    fn client_to_workspace_inserts_at_head() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(&Selector::Index(1)).unwrap(); // 20 -> ws::1
        wm.client_to_workspace(&Selector::Index(1)).unwrap(); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(&Selector::Index(1)).unwrap();

        assert_eq!(
            wm.workspaces[1].iter().collect::<Vec<&WinId>>(),
            vec![&10, &20]
        );
    }

    #[test]
    fn client_to_workspace_sets_focus() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(&Selector::Index(1)).unwrap(); // 20 -> ws::1
        wm.client_to_workspace(&Selector::Index(1)).unwrap(); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(&Selector::Index(1)).unwrap();

        assert_eq!(wm.workspaces[1].focused_client(), Some(10));
    }

    #[test]
    fn client_to_invalid_workspace_is_noop() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
        wm.client_to_workspace(&Selector::Index(42)).unwrap();
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
    }

    #[test]
    fn client_to_screen_sets_correct_workspace() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        wm.client_to_screen(&Selector::Index(1)).unwrap();
        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(1));
    }

    #[test]
    fn client_to_invalid_screen_is_noop() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.client_map.get(&10).map(|c| c.workspace()), Some(0));
        wm.client_to_screen(&Selector::Index(5)).unwrap();
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
        wm.focus_workspace(&Selector::Index(3)).unwrap();
        assert_eq!(wm.workspaces.focused_index(), 3);
        assert_eq!(wm.workspaces.focused_index(), wm.active_ws_index());
    }

    #[test]
    fn dragging_clients_forward_from_index_0() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 5, 0); // focus on last client (50) ix == 0

        let clients = |w: &mut WindowManager<_>| {
            w.workspaces[w.screens[0].wix]
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };

        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.focused_client().unwrap().id(), 50);
        assert_eq!(clients(&mut wm), vec![40, 50, 30, 20, 10]);

        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.focused_client().unwrap().id(), 50);
        assert_eq!(clients(&mut wm), vec![40, 30, 50, 20, 10]);

        wm.client_gained_focus(20);
        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.focused_client().unwrap().id(), 20);
        assert_eq!(clients(&mut wm), vec![40, 30, 50, 10, 20]);
    }

    #[test]
    fn getting_all_clients_on_workspace() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);

        add_n_clients(&mut wm, 3, 0);
        wm.focus_workspace(&Selector::Index(1)).unwrap();
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
        wm.focus_workspace(&Selector::Index(1)).unwrap();
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

    impl ScreenChangingXConn {
        fn set_num_screens(&mut self, n: usize) {
            self.num_screens.set(n);
        }
    }

    impl StubXConn for ScreenChangingXConn {
        fn mock_current_outputs(&self) -> Vec<Screen> {
            let num_screens = self.num_screens.get();
            (0..(num_screens))
                .map(|n| Screen::new(Region::new(800 * n as u32, 600 * n as u32, 800, 600), n))
                .collect()
        }
    }

    impl WindowManager<ScreenChangingXConn> {
        fn set_num_screens(&mut self, n: usize) {
            self.conn_mut().set_num_screens(n);
        }
    }

    #[test]
    fn updating_screens_retains_focused_workspaces() {
        let conn = ScreenChangingXConn {
            num_screens: Cell::new(1),
        };
        let conf = Config::default();
        let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
        wm.init().unwrap();

        // detect_screens is called on init so should have one screen
        assert_eq!(wm.screens.len(), 1);
        assert_eq!(wm.screens.focused_unchecked().wix, 0);

        // Focus workspace 1 the redetect screens: should have 1 and 0
        wm.focus_workspace(&Selector::Index(1)).unwrap();
        assert_eq!(wm.screens.focused_unchecked().wix, 1);
        wm.set_num_screens(2);
        wm.detect_screens().unwrap();
        assert_eq!(wm.screens.len(), 2);
        assert_eq!(wm.screens.get(0).unwrap().wix, 1);
        assert_eq!(wm.screens.get(1).unwrap().wix, 0);

        // Adding another screen should now have WS 2 as 1 is taken
        wm.set_num_screens(3);
        wm.detect_screens().unwrap();
        assert_eq!(wm.screens.len(), 3);
        assert_eq!(wm.screens.get(0).unwrap().wix, 1);
        assert_eq!(wm.screens.get(1).unwrap().wix, 0);
        assert_eq!(wm.screens.get(2).unwrap().wix, 2);

        // Focus WS 3 on screen 1, drop down to 1 screen: it should still have WS 3
        wm.focus_workspace(&Selector::Index(3)).unwrap();
        wm.set_num_screens(1);
        wm.detect_screens().unwrap();
        assert_eq!(wm.screens.len(), 1);
        assert_eq!(wm.screens.get(0).unwrap().wix, 3);
    }
}
