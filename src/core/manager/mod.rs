//! The main user API and control logic for Penrose.
use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
        client::Client,
        config::Config,
        data_types::{Change, Point, Region},
        hooks::Hooks,
        ring::{Direction, InsertPoint, Ring, Selector},
        screen::Screen,
        workspace::Workspace,
        xconnection::{Atom, ClientMessageKind, Prop, WindowState, XConn, Xid},
    },
    ErrorHandler, PenroseError, Result,
};

#[cfg(feature = "serde")]
use crate::core::{helpers::logging_error_handler, layout::LayoutFunc};

use nix::sys::signal::{signal, SigHandler, Signal};
use tracing::Level;

use std::{cell::Cell, collections::HashMap, fmt};

mod event;
mod util;

#[doc(inline)]
pub use event::EventAction;

use event::{process_next_event, WmState};

// Relies on all hooks taking &mut WindowManager as the first arg.
macro_rules! run_hooks {
    ($method:ident, $_self:expr, $($arg:expr),*) => {
        debug!(target: "hooks", "Running {} hooks", stringify!($method));
        let mut hooks = $_self.hooks.replace(vec![]);
        let res = hooks.iter_mut().try_for_each(|h| h.$method($_self, $($arg),*));
        $_self.hooks.replace(hooks);
        if let Err(e) = res {
            ($_self.error_handler)(e);
        }
    };
}

// Mark a result that returns () as being ignored in this case.
macro_rules! ignore_error {
    { $e:expr } => { $e.unwrap_or(()) }
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
    client_map: HashMap<Xid, Client>,
    #[cfg_attr(feature = "serde", serde(skip, default = "default_hooks"))]
    hooks: Cell<Hooks<X>>,
    previous_workspace: usize,
    client_insert_point: InsertPoint,
    focused_client: Option<Xid>,
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

        trace!("building initial workspaces");
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
    #[tracing::instrument(level = "debug", err, skip(self, hooks, error_handler, layout_funcs))]
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
    #[tracing::instrument(level = "debug", err, skip(self))]
    pub fn init(&mut self) -> Result<()> {
        if !self.hydrated {
            panic!("Need to call 'hydrate_and_init' when restoring from serialised state")
        }

        trace!("Initialising XConn");
        self.conn().init()?;

        trace!("Attempting initial screen detection");
        self.detect_screens()?;

        trace!("Setting EWMH properties");
        self.conn.set_wm_properties(&self.config.workspaces)?;

        trace!("Forcing cursor to first screen");
        Ok(self.conn.warp_cursor(None, &self.screens[0])?)
    }

    // Subset of current immutable state that is needed for processing XEvents into EventActions
    fn current_state(&self) -> WmState<'_, X> {
        WmState {
            conn: &self.conn,
            client_map: &self.client_map,
            focused_client: self.focused_client,
        }
    }

    #[tracing::instrument(level = "debug", err, skip(self))]
    pub(crate) fn try_manage_existing_windows(&mut self) -> Result<()> {
        for id in self.conn.active_managed_clients()?.into_iter() {
            trace!(id, "parsing existing client");
            let classes = str_slice!(self.config.floating_classes);
            let mut c = util::parse_existing_client(&self.conn, id, classes)?;
            self.add_client_to_workspace(c.workspace(), id)?;
            self.conn.unmap_client_if_needed(Some(&mut c))?;
            self.client_map.insert(id, c);
            self.conn.mark_new_client(id)?;
        }

        if let Some(id) = self.workspaces[0].focused_client() {
            self.update_focus(id)?;
        }

        self.update_x_known_clients()?;
        self.layout_visible()?;
        Ok(())
    }

    // Each XEvent from the XConn can result in multiple EventActions that need processing
    // depending on the current WindowManager state.
    #[tracing::instrument(level = "trace", err, skip(self, key_bindings, mouse_bindings))]
    fn handle_event_action(
        &mut self,
        action: EventAction,
        key_bindings: &mut KeyBindings<X>,
        mouse_bindings: &mut MouseBindings<X>,
    ) -> Result<()> {
        match action {
            EventAction::ClientFocusGained(id) => self.update_focus(id)?,
            EventAction::ClientFocusLost(id) => self.client_lost_focus(id),
            EventAction::ClientNameChanged(id, is_root) => self.client_name_changed(id, is_root)?,
            EventAction::ClientToWorkspace(id, wix) => self.move_client_to_workspace(id, wix)?,
            EventAction::DestroyClient(id) => self.remove_client(id)?,
            EventAction::DetectScreens => {
                run_hooks!(randr_notify, self,);
                self.detect_screens()?
            }
            EventAction::FocusIn(id) => self.focus_in(id)?,
            EventAction::MapWindow(id) => self.handle_map_request(id)?,
            EventAction::MoveClientIfFloating(id, r) => self.handle_move_if_floating(id, r)?,
            EventAction::RunKeyBinding(k) => self.run_key_binding(k, key_bindings),
            EventAction::RunMouseBinding(e) => self.run_mouse_binding(e, mouse_bindings),
            EventAction::SetActiveClient(id) => self.set_active_client(id)?,
            EventAction::SetActiveWorkspace(wix) => self.focus_workspace(&Selector::Index(wix))?,
            EventAction::SetScreenFromPoint(p) => self.set_screen_from_point(p)?,
            EventAction::ToggleClientFullScreen(id, should_fullscreen) => {
                self.set_fullscreen(id, should_fullscreen)?;
            }
            EventAction::UnknownPropertyChange(id, atom, is_root) => {
                self.handle_prop_change(id, atom, is_root)?;
            }
            EventAction::Unmap(id) => self.handle_unmap_notify(id)?,
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
    /// [1]: crate::core::xconnection::XEventHandler::wait_for_event
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
        trace!("registering SIGCHILD signal handler");
        if let Err(e) = unsafe { signal(Signal::SIGCHLD, SigHandler::SigIgn) } {
            panic!("unable to set signal handler: {}", e);
        }

        trace!("grabbing key and mouse bindings");
        self.conn.grab_keys(&key_bindings, &mouse_bindings)?;

        trace!("forcing focus to first workspace");
        self.focus_workspace(&Selector::Index(0))?;

        run_hooks!(startup, self,);
        self.running = true;

        trace!("entering main event loop");
        while self.running {
            match self.conn.wait_for_event() {
                Ok(event) => {
                    let span = span!(target: "penrose", Level::DEBUG, "XEvent", %event);
                    let _enter = span.enter();
                    trace!(details = ?event, "event details");

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

                Err(e) => (self.error_handler)(PenroseError::X(e)),
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
    fn workspace_index_for_client(&mut self, id: Xid) -> Option<usize> {
        self.client_map.get(&id).map(|c| c.workspace())
    }

    /// The [Xid] of the client that currently has focus.
    ///
    /// Returns `None` if there are no clients to focus.
    pub fn focused_client_id(&self) -> Option<Xid> {
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

    // Set X focus to the requested client if it accepts focus, otherwise send a
    // 'take focus' event for the client to process
    fn set_focus(&self, id: Xid, accepts_focus: bool) -> Result<()> {
        trace!(id, accepts_focus, "setting focus");
        Ok(if accepts_focus {
            if let Err(e) = self.conn.focus_client(id) {
                warn!("unable to focus client {}: {}", id, e);
            }
            self.conn.change_prop(
                self.conn.root(),
                Atom::NetActiveWindow.as_ref(),
                Prop::Window(vec![id]),
            )?;
            let fb = self.config.focused_border;
            if let Err(e) = self.conn.set_client_border_color(id, fb) {
                warn!("unable to set client border color for {}: {}", id, e);
            }
        } else {
            let msg = ClientMessageKind::TakeFocus(id).as_message(&self.conn)?;
            self.conn.send_client_event(msg)?;
        })
    }

    fn focus_in(&self, id: Xid) -> Result<()> {
        let accepts_focus = match self.client_map.get(&id) {
            Some(client) => client.accepts_focus,
            None => self.conn.client_accepts_focus(id),
        };
        self.set_focus(id, accepts_focus)
    }

    // Set the current focus point based on client focus hints
    #[tracing::instrument(level = "trace", err, skip(self))]
    fn update_focus(&mut self, id: Xid) -> Result<()> {
        let target = if self.client_map.contains_key(&id) {
            id
        } else {
            // Try to fallback to the focused_client on the active workspace if this ID is unknown to us
            // FIXME: Is this behaviour correct? We might want to instead try to pull the details
            //        of this client and add it to the client_map. Not if we ever hit this case or
            //        not, and if we do, why we do...
            warn!(id, "An unknown client has gained focus");
            match self.active_workspace().focused_client() {
                Some(id) => id,

                // The requested id wasn't something we know about and we don't have any clients on the
                // active workspace so all we can do is drop our focused state and revert focus back to
                // the root window.
                None => {
                    let root = self.conn.root();
                    if let Err(e) = self.conn.focus_client(root) {
                        warn!("unable to focus root window: {}", e);
                    }
                    let active_window = Atom::NetActiveWindow.as_ref();
                    self.conn.delete_prop(root, active_window)?;
                    run_hooks!(focus_change, self, root);
                    return Ok(());
                }
            }
        };

        let prev = self.focused_client_id();
        self.focused_client = Some(target);
        if prev.is_some() && Some(target) != prev {
            prev.map(|prev_id| self.client_lost_focus(prev_id));
        }

        let (wix, accepts_focus) = {
            // Safe to unwrap because we make sure this is a known client above
            let c = self.client_map.get(&target).unwrap();
            (c.workspace(), c.accepts_focus)
        };

        self.focus_screen(&Selector::Condition(&|s| s.wix == wix));
        self.set_focus(target, accepts_focus)?;

        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.focus_client(target);
            let in_ws = prev.map_or(false, |prev_id| ws.client_ids().contains(&prev_id));
            if ws.layout_conf().follow_focus && in_ws {
                if let Err(e) = self.apply_layout(wix) {
                    error!("unable to apply layout on ws {}: {}", wix, e);
                }
            }
        }

        run_hooks!(focus_change, self, target);
        Ok(())
    }

    // The given X window ID lost focus according to the X server
    #[tracing::instrument(level = "trace", skip(self))]
    fn client_lost_focus(&mut self, id: Xid) {
        if self.focused_client == Some(id) {
            self.focused_client = None;
        }

        if self.client_map.contains_key(&id) {
            let ub = self.config.unfocused_border;
            // The target window may have lost focus because it has just been closed and
            // we have not yet updated our state.
            ignore_error!(self.conn.set_client_border_color(id, ub));
        }
    }

    // The given window ID has had its EWMH name updated by something
    fn client_name_changed(&mut self, id: Xid, is_root: bool) -> Result<()> {
        let name = self.conn.client_name(id)?;
        if !is_root {
            if let Some(c) = self.client_map.get_mut(&id) {
                c.set_name(&name)
            }
        }
        run_hooks!(client_name_updated, self, id, &name, is_root);
        Ok(())
    }

    // The given window ID has been destroyed so remove our internal state referencing it.
    #[tracing::instrument(level = "trace", err, skip(self))]
    fn remove_client(&mut self, id: Xid) -> Result<()> {
        if let Some(client) = self.client_map.remove(&id) {
            let wix = client.workspace();
            self.workspaces.apply_to(&Selector::Index(wix), |ws| {
                ws.remove_client(id);
            });

            if self.focused_client == Some(id) {
                self.focused_client = None;
            }

            if self.visible_workspaces().contains(&wix) {
                self.apply_layout(wix)?;
            }

            self.update_x_known_clients()?;
            run_hooks!(remove_client, self, id);
        } else {
            debug!(id, "attempt to remove unknown client");
        }

        Ok(())
    }

    #[tracing::instrument(level = "trace", err, skip(self))]
    fn move_client_to_workspace(&mut self, id: Xid, wix: usize) -> Result<()> {
        if !self.client_map.contains_key(&id) {
            return Err(PenroseError::UnknownClient(id));
        }

        // We know we have the client at this point so unwrap is fine
        let current_wix = self.workspace_index_for_client(id).unwrap();
        if current_wix != wix {
            self.workspaces
                .get_mut(current_wix)
                .and_then(|ws| ws.remove_client(id));
            self.add_client_to_workspace(wix, id)?;
            self.client_map
                .entry(id)
                .and_modify(|c| c.set_workspace(wix));

            if self.visible_workspaces().contains(&wix) {
                let s = self.screens.focused_unchecked();
                self.conn.warp_cursor(Some(id), s)?;
            } else {
                self.conn
                    .unmap_client_if_needed(self.client_map.get_mut(&id))?;
            }

            self.layout_visible()?;
        }

        Ok(())
    }

    #[tracing::instrument(level = "trace", err, skip(self))]
    fn layout_visible(&mut self) -> Result<()> {
        for wix in self.visible_workspaces() {
            self.apply_layout(wix)?;
        }
        Ok(())
    }

    /// Query the [XConn] for the current connected [Screen] list and reposition displayed
    /// [Workspace] instances if needed.
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
    #[tracing::instrument(level = "trace", err, skip(self))]
    pub fn detect_screens(&mut self) -> Result<()> {
        let screens = util::get_screens(
            &self.conn,
            self.visible_workspaces(),
            self.workspaces.len(),
            self.config.bar_height,
            self.config.top_bar,
        )?;

        if screens == self.screens.as_vec() {
            return Ok(()); // nothing changed
        }

        info!(n = screens.len(), "updating known screens");
        self.screens = Ring::new(screens);
        for wix in self.visible_workspaces() {
            self.apply_layout(wix)?;
        }

        let regions = self.screens.vec_map(|s| s.region(false));
        run_hooks!(screens_updated, self, &regions);

        Ok(())
    }

    // Map a new client window.
    #[tracing::instrument(level = "trace", err, skip(self))]
    fn handle_map_request(&mut self, id: Xid) -> Result<()> {
        trace!(id, "handling map request");
        let classes = str_slice!(self.config.floating_classes);
        let mut client = Client::new(&self.conn, id, self.active_ws_index(), classes);
        trace!(id, ?client.wm_name, ?client.wm_class, ?client.wm_type, "client details");

        // Run hooks to allow them to modify the client
        run_hooks!(new_client, self, &mut client);

        if !self.conn.is_managed_client(id) {
            return Ok(self.conn.map_client(id)?);
        }

        let wix = client.workspace();

        if client.wm_managed {
            self.add_client_to_workspace(wix, id)?;
        }

        if client.floating {
            if let Some((_, s)) = self.indexed_screen_for_workspace(wix) {
                util::position_floating_client(
                    &self.conn,
                    id,
                    s.region(self.config.show_bar),
                    self.config.border_px,
                )?
            }
        }

        self.client_map.insert(id, client);
        self.conn.mark_new_client(id)?;
        self.update_focus(id)?;
        self.update_x_known_clients()?;

        if wix == self.active_ws_index() {
            self.apply_layout(wix)?;
            self.conn
                .map_client_if_needed(self.client_map.get_mut(&id))?;
            let s = self.screens.focused_unchecked();
            self.conn.warp_cursor(Some(id), s)?;
        }

        Ok(())
    }

    fn handle_move_if_floating(&mut self, id: Xid, r: Region) -> Result<()> {
        if let Some(client) = self.client_map.get(&id) {
            if client.floating {
                debug!(id, region = ?r, "repositioning floating window");
                let bpx = self.config.border_px;
                self.conn.position_client(id, r, bpx, true)?;
            }
        }
        Ok(())
    }

    fn handle_prop_change(&mut self, id: Xid, atom: String, is_root: bool) -> Result<()> {
        trace!(id, is_root, ?atom, "dropping prop change (unimplemented)");
        Ok(())
    }

    fn handle_unmap_notify(&mut self, id: Xid) -> Result<()> {
        Ok(self.conn.set_client_state(id, WindowState::Withdrawn)?)
    }

    // NOTE: This defers control of the [WindowManager] to the user's key-binding action
    //       which can lead to arbitrary calls to public methods on the [WindowManager]
    //       including mutable methods.
    #[tracing::instrument(level = "debug", skip(self, k, bindings), fields(k.code, k.mask))]
    fn run_key_binding(&mut self, k: KeyCode, bindings: &mut KeyBindings<X>) {
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
    #[tracing::instrument(level = "debug", skip(self, e, bindings), fields(?e.state, ?e.kind))]
    fn run_mouse_binding(&mut self, e: MouseEvent, bindings: &mut MouseBindings<X>) {
        if let Some(action) = bindings.get_mut(&(e.kind, e.state.clone())) {
            // ignoring Child handlers and SIGCHILD
            if let Err(e) = action(self, &e) {
                (self.error_handler)(e);
            }
        }
    }

    fn set_active_client(&mut self, id: Xid) -> Result<()> {
        self.focus_client(&Selector::WinId(id))
            .map_err(|_| PenroseError::UnknownClient(id))?;

        Ok(())
    }

    // Set the active [Screen] based on an (x, y) [Point]. If point is None then we set
    // based on the current cursor position instead.
    fn set_screen_from_point(&mut self, point: Option<Point>) -> Result<()> {
        let point = match point {
            Some(p) => p,
            None => self.conn.cursor_position()?,
        };
        self.focus_screen(&Selector::Condition(&|s: &Screen| s.contains(point)));
        Ok(())
    }

    // Toggle the given client fullscreen. This has knock on effects for other windows and can
    // be triggered by user key bindings as well as applications requesting full screen as well.
    // TODO: should something going fullscreen also hide unmaged windows?
    fn set_fullscreen(&mut self, id: Xid, should_fullscreen: bool) -> Result<()> {
        let (currently_fullscreen, wix) = self
            .client_map
            .get_mut(&id)
            .map(|c| (c.fullscreen, c.workspace()))
            .ok_or(PenroseError::UnknownClient(id))?;

        if currently_fullscreen == should_fullscreen {
            return Ok(()); // Client is already in the correct state, we shouldn't have been called
        }

        let r = match self.screen(&Selector::Condition(&|s| s.wix == wix)) {
            Some(s) => s.region(false),
            None => return Ok(()),
        };

        let workspace = self
            .workspaces
            .get_mut(wix)
            .ok_or_else(|| perror!("unknown workspace: {}", wix))?;

        if util::toggle_fullscreen(&self.conn, id, &mut self.client_map, workspace, r)? {
            self.apply_layout(wix)?;
        }

        Ok(())
    }

    /*
     * Common mid level actions that make up larger event response handlers.
     */

    #[tracing::instrument(level = "debug", err, skip(self))]
    fn apply_layout(&mut self, wix: usize) -> Result<()> {
        let ws = match self.workspaces.get(wix) {
            Some(ws) => ws,
            None => {
                return Err(perror!("attempt to layout unknown workspace: {}", wix));
            }
        };

        let (i, s) = match self.indexed_screen_for_workspace(wix) {
            Some(index_and_screen) => index_and_screen,
            None => return Ok(()), // workspace is not currently visible
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
            )?;
        }

        run_hooks!(layout_applied, self, wix, i);
        Ok(())
    }

    fn update_x_workspace_details(&mut self) -> Result<()> {
        let names = self.workspaces.vec_map(|w| w.name().to_string());
        self.conn.update_desktops(&names)?;
        run_hooks!(
            workspaces_updated,
            self,
            str_slice!(names),
            self.active_ws_index()
        );

        Ok(())
    }

    fn update_x_known_clients(&self) -> Result<()> {
        let clients: Vec<Xid> = self.client_map.keys().copied().collect();
        Ok(self.conn.update_known_clients(&clients)?)
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

    #[tracing::instrument(level = "trace", err, skip(self))]
    fn add_client_to_workspace(&mut self, wix: usize, id: Xid) -> Result<()> {
        self.client_map
            .entry(id)
            .and_modify(|c| c.set_workspace(wix));
        let cip = self.client_insert_point;
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.add_client(id, &cip)?;
            self.conn.set_client_workspace(id, wix)?;
            run_hooks!(client_added_to_workspace, self, id, wix);
        };

        Ok(())
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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.log("hello from penrose!")?;
    /// manager.log(format!("This manager has {} screens", manager.n_screens()))?;
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn log(&self, msg: impl Into<String>) -> Result<()> {
        let msg = msg.into();
        info!("{}", msg);

        Ok(())
    }

    /// Cycle between known [screens][Screen]. Does not wrap from first to last
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
                .warp_cursor(None, self.screens.focused_unchecked())?;
            let wix = self.workspaces.focused_index();
            self.conn.set_current_workspace(wix)?;

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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
            self.update_focus(new)?;
            let screen = self.screens.focused_unchecked();
            self.conn.warp_cursor(Some(new), screen)?;
        }

        Ok(())
    }

    /// Focus the [Client] matching the given [Selector]
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let focused = manager.focus_client(&Selector::WinId(0));
    /// assert_eq!(focused.unwrap(), 0);
    ///
    /// let focused = manager.focus_client(&Selector::WinId(42));
    /// assert!(focused.is_err());
    ///
    /// let focused = manager.focus_client(&Selector::WinId(1));
    /// assert_eq!(focused.unwrap(), 1);
    ///
    /// let focused = manager.focus_client(&Selector::WinId(42));
    /// assert!(focused.is_err());
    /// # Ok(())
    /// # }
    /// #
    /// # fn example2(mut manager: ExampleWM) -> Result<()> {
    ///
    /// // Or, if there are no clients to focus
    /// let focused = manager.focus_client(&Selector::WinId(0));
    /// assert!(focused.is_err());
    /// # Ok(())
    /// # }
    /// #
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), HashMap::new()).unwrap();
    /// # example(manager).unwrap();
    /// # example2(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn focus_client(&mut self, selector: &Selector<'_, Client>) -> Result<Xid> {
        let id = match self.client(selector) {
            Some(c) => c.id(),
            None => return Err(PenroseError::NoMatchingElement),
        };
        self.update_focus(id)?;
        let screen = self.screens.focused_unchecked();
        self.conn.warp_cursor(Some(id), screen)?;
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
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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

        self.apply_layout(wix)
    }

    /// Move the focused [Client] through the stack of clients on the active [Workspace].
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
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
            self.apply_layout(wix)?;
            self.update_focus(id)?;
            self.conn
                .warp_cursor(Some(id), self.screens.focused_unchecked())?;
        }

        Ok(())
    }

    /// Cycle between [layouts][1] for the active [Workspace]
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.current_layout_symbol(), "first");
    ///
    /// manager.cycle_layout(Forward)?;
    /// assert_eq!(manager.current_layout_symbol(), "second");
    ///
    /// // Wrap at the end of the layout list
    /// manager.cycle_layout(Forward)?;
    /// assert_eq!(manager.current_layout_symbol(), "first");
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
        self.apply_layout(wix)?;

        Ok(())
    }

    /// Increase or decrease the number of clients in the main area by 1.
    ///
    /// The change is applied to the active [layout][1] on the [Workspace] that currently holds
    /// focus.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.update_max_main(More);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    ///
    /// [1]: crate::core::layout::Layout
    pub fn update_max_main(&mut self, change: Change) -> Result<()> {
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.update_max_main(change)
        };
        self.apply_layout(wix)?;

        Ok(())
    }

    /// Increase or decrease the current [layout][crate::core::layout::Layout] main_ratio by
    /// `main_ratio_step`
    ///
    /// The change is applied to the active [layout][1] on the [Workspace] that currently holds
    /// focus.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.update_main_ratio(More);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    ///
    /// [1]: crate::core::layout::Layout
    pub fn update_main_ratio(&mut self, change: Change) -> Result<()> {
        let step = self.config.main_ratio_step;
        let wix = self.active_ws_index();
        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.update_main_ratio(change, step)
        }
        self.apply_layout(wix)?;

        Ok(())
    }

    /// Shut down the WindowManager, running any required cleanup and exiting penrose
    ///
    /// **NOTE**: any registered hooks on the `WindowManager` will still run following calling this
    /// method, with the actual exit condition being checked and handled at the end.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.exit();
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn exit(&mut self) -> Result<()> {
        self.conn.cleanup()?;
        self.conn.flush();
        self.running = false;

        Ok(())
    }

    /// The layout symbol for the [layout][1] currently being used on the
    /// active workspace
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.current_layout_symbol(), "first");
    ///
    /// manager.cycle_layout(Forward)?;
    /// assert_eq!(manager.current_layout_symbol(), "second");
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    ///
    /// [1]: crate::core::layout::Layout
    pub fn current_layout_symbol(&self) -> &str {
        match self.workspaces.get(self.active_ws_index()) {
            Some(ws) => ws.layout_symbol(),
            None => "???",
        }
    }

    /// Set the root X window name. Useful for exposing information to external programs
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.set_root_window_name("hello world")?;
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn set_root_window_name(&self, s: impl AsRef<str>) -> Result<()> {
        Ok(self.conn.set_root_window_name(s.as_ref())?)
    }

    /// Set the insert point for new clients. Default is to insert at index 0.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// // Starting with three clients that have been inserted via InsertPoint::First
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    ///
    /// // Move them all over to another workspace, still using InsertPoint::First
    /// (0..3).try_for_each(|_| manager.client_to_workspace(&Selector::Index(1)));
    /// manager.focus_workspace(&Selector::Index(1))?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![0, 1, 2]);
    ///
    /// // Change to InsertPoint::Last and move them back
    /// manager.set_client_insert_point(InsertPoint::Last)?;
    ///
    /// (0..3).try_for_each(|_| manager.client_to_workspace(&Selector::Index(0)));
    /// manager.focus_workspace(&Selector::Index(0))?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![0, 1, 2]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn set_client_insert_point(&mut self, cip: InsertPoint) -> Result<()> {
        self.client_insert_point = cip;

        Ok(())
    }

    /// Set the displayed workspace for the focused screen to be `index` in the list of
    /// workspaces passed at `init`.
    ///
    /// A common way to use this method is in a `refMap` section when generating your keybindings
    /// and using the [index_selectors][1] helper method to make the required [selectors][2].
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().name(), "1");
    ///
    /// manager.focus_workspace(&Selector::Index(3))?;
    /// assert_eq!(manager.active_workspace().name(), "4");
    ///
    /// manager.focus_workspace(&Selector::Condition(&|ws| ws.name() == "9"))?;
    /// assert_eq!(manager.active_workspace().name(), "9");
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    ///
    /// [1]: crate::core::helpers::index_selectors
    /// [2]: crate::core::ring::Selector
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
                    self.apply_layout(active)?;
                    self.apply_layout(index)?;

                    let ws = self.workspaces.get(index);
                    if let Some(id) = ws.and_then(|ws| ws.focused_client()) {
                        self.update_focus(id)?;
                    };

                    self.workspaces.focus(&Selector::Index(index));
                    run_hooks!(workspace_change, self, active, index);
                    return Ok(());
                }
            }

            // target not currently displayed so unmap what we currently have
            // displayed and replace it with the target workspace
            if let Some(ws) = self.workspaces.get(active) {
                for id in ws.client_ids().iter() {
                    self.conn
                        .unmap_client_if_needed(self.client_map.get_mut(id))?
                }
            }

            if let Some(ws) = self.workspaces.get(index) {
                for id in ws.client_ids().iter() {
                    self.conn
                        .map_client_if_needed(self.client_map.get_mut(id))?;
                }
            }

            self.screens.focused_mut_unchecked().wix = index;
            self.apply_layout(index)?;
            self.conn.set_current_workspace(index)?;

            let ws = self.workspaces.get(index);
            if let Some(id) = ws.and_then(|ws| ws.focused_client()) {
                self.update_focus(id)?;
            };

            self.workspaces.focus(&Selector::Index(index));
            run_hooks!(workspace_change, self, active, index);
        }

        Ok(())
    }

    /// Switch focus back to the last workspace that had focus.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.focus_workspace(&Selector::Index(1))?;
    /// assert_eq!(manager.active_workspace().name(), "2");
    ///
    /// manager.focus_workspace(&Selector::Index(0))?;
    /// assert_eq!(manager.active_workspace().name(), "1");
    ///
    /// manager.toggle_workspace()?;
    /// assert_eq!(manager.active_workspace().name(), "2");
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn toggle_workspace(&mut self) -> Result<()> {
        self.focus_workspace(&Selector::Index(self.previous_workspace))
    }

    /// Move the focused client to the workspace matching 'selector'.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    ///
    /// (0..3).try_for_each(|_| manager.client_to_workspace(&Selector::Index(1)));
    /// manager.focus_workspace(&Selector::Index(1))?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![0, 1, 2]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn client_to_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Result<()> {
        if let Some(id) = self.focused_client {
            if let Some(wix) = self.workspaces.index(selector) {
                self.move_client_to_workspace(id, wix)?;
                self.focused_client = self.active_workspace().focused_client();
            }
        }

        Ok(())
    }

    /// Move the focused client to the active workspace on the screen matching 'selector'.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.focused_workspaces(), vec![0, 1]);
    ///
    /// assert_eq!(manager.active_screen_index(), 0);
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    ///
    /// manager.client_to_screen(&Selector::Index(1))?;
    /// assert_eq!(manager.active_workspace().client_ids(), vec![1, 0]);
    ///
    /// manager.cycle_screen(Forward)?;
    /// assert_eq!(manager.active_screen_index(), 1);
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(2, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn client_to_screen(&mut self, selector: &Selector<'_, Screen>) -> Result<()> {
        let i = match self.screen(selector) {
            Some(s) => s.wix,
            None => return Ok(()),
        };
        self.client_to_workspace(&Selector::Index(i))
    }

    /// Toggle the fullscreen state of the [Client] matching the given [Selector]
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.client(&Selector::Focused).unwrap().is_fullscreen(), false);
    ///
    /// manager.toggle_client_fullscreen(&Selector::Focused)?;
    /// assert_eq!(manager.client(&Selector::Focused).unwrap().is_fullscreen(), true);
    ///
    /// manager.toggle_client_fullscreen(&Selector::Focused)?;
    /// assert_eq!(manager.client(&Selector::Focused).unwrap().is_fullscreen(), false);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(1));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn toggle_client_fullscreen(&mut self, selector: &Selector<'_, Client>) -> Result<()> {
        let (id, client_is_fullscreen) = match self.client(selector) {
            None => return Ok(()), // unknown client
            Some(c) => (c.id(), c.fullscreen),
        };
        self.set_fullscreen(id, !client_is_fullscreen)
    }

    /// Kill the focused client window.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// manager.kill_client()?;
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(level = "debug", err, skip(self))]
    pub fn kill_client(&mut self) -> Result<()> {
        if let Some(id) = self.focused_client {
            // let del = Atom::WmDeleteWindow.as_ref();
            // let res = if let Ok(true) = self.conn.client_supports_protocol(id, del) {
            //     trace!(id, "client supports WmDeleteWindow: sending client event");
            //     ClientMessageKind::DeleteWindow(id)
            //         .as_message(&self.conn)
            //         .and_then(|msg| self.conn.send_client_event(msg))
            //         .or(self.conn.destroy_client(id))
            // } else {
            //     trace!(id, "client doesn't supports WmDeleteWindow: destroying");
            //     self.conn.destroy_client(id)
            // };

            trace!(id, "sending destroy");
            let res = self.conn.destroy_client(id);
            if let Err(e) = res {
                error!(id, "error killing client: {}", e);
            }

            self.conn.flush();
        }

        Ok(())
    }

    /// Get a reference to the first Screen satisfying 'selector'. Xid selectors will return
    /// the screen containing that Client if the client is known.
    /// NOTE: It is not possible to get a mutable reference to a Screen.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    /// assert_eq!(manager.focused_workspaces(), vec![0, 1]);
    ///
    /// assert_eq!(
    ///     manager.screen(&Selector::Focused),
    ///     manager.screen(&Selector::Index(manager.active_screen_index()))
    /// );
    ///
    /// assert_eq!(
    ///     manager.screen(&Selector::Index(0)),
    ///     manager.screen(&Selector::WinId(0)),
    /// );
    ///
    /// manager.client_to_screen(&Selector::Index(1))?;
    ///
    /// assert_eq!(
    ///     manager.screen(&Selector::WinId(2)),
    ///     manager.screen(&Selector::Index(1)),
    /// );
    ///
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(2, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().name(), "1");
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn active_workspace(&self) -> &Workspace {
        self.workspaces
            .element(&Selector::Index(self.active_ws_index()))
            .unwrap()
    }

    /// A mutable reference to the current active [Workspace]
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    ///
    /// manager.active_workspace_mut().rotate_clients(Forward);
    /// assert_eq!(manager.active_workspace().client_ids(), vec![0, 2, 1]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        self.workspaces
            .element_mut(&Selector::Index(self.active_ws_index()))
            .unwrap()
    }

    /// The currently focused workspace indices being shown on each screen
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.focused_workspaces(), vec![0, 1]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(2, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn focused_workspaces(&self) -> Vec<usize> {
        self.screens.iter().map(|s| s.wix).collect()
    }

    /// Add a new workspace at `index`, shifting all workspaces with indices greater to the right.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let names: Vec<_> = manager
    ///     .all_workspaces(&Selector::Any)
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(names, vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    ///
    /// let ws = Workspace::new("new", example_layouts());
    /// manager.add_workspace(1, ws)?;
    ///
    /// let new_names: Vec<_> = manager
    ///     .all_workspaces(&Selector::Any)
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(new_names, vec!["1", "new", "2", "3", "4", "5", "6", "7", "8", "9"]);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn add_workspace(&mut self, index: usize, ws: Workspace) -> Result<()> {
        self.workspaces.insert(index, ws);
        self.update_x_workspace_details()?;

        Ok(())
    }

    /// Add a new workspace at the end of the current workspace list
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let names: Vec<_> = manager
    ///     .all_workspaces(&Selector::Any)
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(names, vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    ///
    /// let ws = Workspace::new("new", example_layouts());
    /// manager.push_workspace(ws)?;
    ///
    /// let new_names: Vec<_> = manager
    ///     .all_workspaces(&Selector::Any)
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(new_names, vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "new"]);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn push_workspace(&mut self, ws: Workspace) -> Result<()> {
        self.workspaces.push(ws);
        self.update_x_workspace_details()?;

        Ok(())
    }

    /// Remove a Workspace from the WindowManager. All clients that were present on the removed
    /// workspace will be destroyed. Xid selectors will be ignored.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let names: Vec<_> = manager
    ///     .all_workspaces(&Selector::Any)
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(names, vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    ///
    /// let removed = manager.remove_workspace(&Selector::Index(2))?;
    /// assert!(removed.is_some());
    /// assert_eq!(removed.unwrap().name(), "3");
    ///
    /// let new_names: Vec<_> = manager
    ///     .all_workspaces(&Selector::Any)
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(new_names, vec!["1", "2", "4", "5", "6", "7", "8", "9"]);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn remove_workspace(
        &mut self,
        selector: &Selector<'_, Workspace>,
    ) -> Result<Option<Workspace>> {
        if self.workspaces.len() == self.screens.len() {
            return Err(perror!("must have at least one workspace per screen"));
        }

        let ws = self
            .workspaces
            .remove(&selector)
            .ok_or_else(|| perror!("unknown workspace"))?;
        ws.iter().try_for_each(|c| self.remove_client(*c))?;

        // Focus the workspace before the one we just removed. There is always at least one
        // workspace before this one due to the guard above.
        let ix = self.screens.focused_unchecked().wix.saturating_sub(1);
        self.focus_workspace(&Selector::Index(ix))?;
        self.update_x_workspace_details()?;

        Ok(Some(ws))
    }

    /// Get a reference to the first Workspace satisfying 'selector'. Xid selectors will return
    /// the workspace containing that Client if the client is known.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.workspace(&Selector::Focused).unwrap().name(), "1");
    /// assert_eq!(manager.workspace(&Selector::Index(3)).unwrap().name(), "4");
    ///
    /// assert_eq!(
    ///     manager.workspace(&Selector::Focused).unwrap().client_ids(),
    ///     vec![2, 1, 0]
    /// );
    ///
    /// manager.client_to_workspace(&Selector::Index(2))?;
    /// assert_eq!(manager.workspace(&Selector::Index(2)).unwrap().name(), "3");
    /// assert_eq!(manager.workspace(&Selector::WinId(0)).unwrap().name(), "1");
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn workspace(&self, selector: &Selector<'_, Workspace>) -> Option<&Workspace> {
        if let Selector::WinId(id) = selector {
            self.client_map
                .get(&id)
                .and_then(|c| self.workspaces.get(c.workspace()))
        } else {
            self.workspaces.element(&selector)
        }
    }

    /// Get a mutable reference to the first Workspace satisfying 'selector'. Xid selectors will
    /// return the workspace containing that Client if the client is known.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().client_ids(), vec![2, 1, 0]);
    ///
    /// let ws2 = Selector::Index(2);
    ///
    /// manager.client_to_workspace(&ws2)?;
    /// manager.client_to_workspace(&ws2)?;
    /// manager.client_to_workspace(&ws2)?;
    ///
    /// assert_eq!(manager.workspace(&ws2).map(|w| w.client_ids()), Some(vec![0, 1, 2]));
    ///
    /// manager.workspace_mut(&ws2).map(|w| w.rotate_clients(Forward));
    /// assert_eq!(manager.workspace(&ws2).map(|w| w.client_ids()), Some(vec![2, 0, 1]));
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
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

    /// Get a vector of immutable references to _all_ workspaces that match the provided [Selector].
    ///
    /// To return only a single workspace in the case that a selector matches multiple workspaces,
    /// use the [workspace][1] method instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let names: Vec<&str> = manager
    ///     .all_workspaces(&Selector::Condition(&|ws|
    ///         ws.name().parse::<usize>().unwrap() < 5
    ///     ))
    ///     .iter()
    ///     .map(|w| w.name())
    ///     .collect();
    ///
    /// assert_eq!(names, vec!["1", "2", "3", "4"]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(1));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    ///
    /// [1]: crate::core::manager::WindowManager::workspace
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

    /// Get a vector of mutable references to _all_ workspaces that match the provided [Selector].
    ///
    /// To return only a single workspace in the case that a selector matches multiple workspaces,
    /// use the [workspace_mut][1] method instead.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let layouts: Vec<&str> = manager
    ///     .all_workspaces(&Selector::Condition(&|ws|
    ///         ws.name().parse::<usize>().unwrap() < 3
    ///     ))
    ///     .iter()
    ///     .map(|w| w.layout_symbol())
    ///     .collect();
    ///
    /// assert_eq!(layouts, vec!["first", "first"]);
    ///
    /// manager
    ///     .all_workspaces_mut(&Selector::Condition(&|ws|
    ///         ws.name().parse::<usize>().unwrap() < 3
    ///     ))
    ///     .iter_mut()
    ///     .for_each(|ws| {
    ///         ws.try_set_layout("second");
    ///     });
    ///
    /// let layouts: Vec<&str> = manager
    ///     .all_workspaces(&Selector::Condition(&|ws|
    ///         ws.name().parse::<usize>().unwrap() < 5
    ///     ))
    ///     .iter()
    ///     .map(|w| w.layout_symbol())
    ///     .collect();
    ///
    /// assert_eq!(layouts, vec!["second", "second", "first", "first"]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(1));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    ///
    /// [1]: crate::core::manager::WindowManager::workspace_mut
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
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_workspace().name(), "1");
    ///
    /// manager.set_workspace_name("foo", &Selector::Focused)?;
    /// assert_eq!(manager.active_workspace().name(), "foo");
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(1, vec![])).unwrap();
    /// ```
    pub fn set_workspace_name(
        &mut self,
        name: impl Into<String>,
        selector: &Selector<'_, Workspace>,
    ) -> Result<()> {
        if let Some(ws) = self.workspaces.element_mut(&selector) {
            ws.set_name(name)
        };
        self.update_x_workspace_details()?;

        Ok(())
    }

    /// Take a reference to the first Client found matching 'selector'
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.client(&Selector::Focused).unwrap().id(), 2);
    /// assert_eq!(manager.client(&Selector::Index(2)).unwrap().id(), 0);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn client(&self, selector: &Selector<'_, Client>) -> Option<&Client> {
        match selector {
            Selector::Focused | Selector::Any => self.focused_client(),
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
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.client(&Selector::Focused).map(|c| c.workspace()), Some(0));
    ///
    /// manager.client_mut(&Selector::Focused).map(|c| c.set_workspace(5));
    /// assert_eq!(manager.client(&Selector::Focused).map(|c| c.workspace()), Some(5));
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn client_mut(&mut self, selector: &Selector<'_, Client>) -> Option<&mut Client> {
        match selector {
            Selector::Focused | Selector::Any => self.focused_client_mut(),
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
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let all_ids: Vec<Xid> = manager
    ///     .all_clients(&Selector::Any)
    ///     .iter()
    ///     .map(|c| c.id())
    ///     .collect();
    ///
    /// assert_eq!(all_ids, vec![0, 1, 2, 3, 4, 5]);
    ///
    /// let ids: Vec<Xid> = manager
    ///     .all_clients(&Selector::Condition(&|c| c.id() > 3))
    ///     .iter()
    ///     .map(|c| c.id())
    ///     .collect();
    ///
    /// assert_eq!(ids, vec![4, 5]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(6));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    pub fn all_clients(&self, selector: &Selector<'_, Client>) -> Vec<&Client> {
        let mut clients: Vec<&Client> = match selector {
            Selector::Any => self.client_map.values().collect(),
            Selector::Focused => self.focused_client().into_iter().collect(),
            Selector::WinId(id) => self.client_map.get(&id).into_iter().collect(),
            Selector::Condition(f) => self.client_map.values().filter(|v| f(v)).collect(),
            Selector::Index(i) => self
                .workspaces
                .get(self.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
                .and_then(|id| self.client_map.get(id))
                .into_iter()
                .collect(),
        };

        clients.sort_unstable_by_key(|c| c.id());
        clients
    }

    /// Get a vector of mutable references to the Clients found matching 'selector'.
    ///
    /// The resulting vector is sorted by Client id.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// let workspace_ids: Vec<usize> = manager
    ///     .all_clients(&Selector::Any)
    ///     .iter()
    ///     .map(|c| c.workspace())
    ///     .collect();
    ///
    /// assert_eq!(workspace_ids, vec![0, 0, 0, 0, 0, 0]);
    ///
    /// manager
    ///     .all_clients_mut(&Selector::Condition(&|c| c.id() > 3))
    ///     .iter_mut()
    ///     .for_each(|c| c.set_workspace(5));
    ///
    /// let workspace_ids: Vec<usize> = manager
    ///     .all_clients(&Selector::Any)
    ///     .iter()
    ///     .map(|c| c.workspace())
    ///     .collect();
    ///
    /// assert_eq!(workspace_ids, vec![0, 0, 0, 0, 5, 5]);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(1, n_clients(6));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    pub fn all_clients_mut(&mut self, selector: &Selector<'_, Client>) -> Vec<&mut Client> {
        let mut clients: Vec<&mut Client> = match selector {
            Selector::Any => self.client_map.values_mut().collect(),
            Selector::Focused => self.focused_client_mut().into_iter().collect(),
            Selector::WinId(id) => self.client_map.get_mut(&id).into_iter().collect(),
            Selector::Condition(f) => self.client_map.values_mut().filter(|v| f(v)).collect(),
            Selector::Index(i) => {
                match self
                    .workspaces
                    .get(self.active_ws_index())
                    .and_then(|ws| ws.iter().nth(*i))
                {
                    Some(id) => self.client_map.get_mut(id).into_iter().collect(),
                    None => vec![],
                }
            }
        };

        clients.sort_unstable_by_key(|c| c.id());
        clients
    }

    /// The number of detected screens currently being tracked by the WindowManager.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.n_screens(), 2);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(2, vec![])).unwrap();
    /// ```
    pub fn n_screens(&self) -> usize {
        self.screens.len()
    }

    /// The current effective screen size of the target screen. Effective screen size is the
    /// physical screen size minus any space reserved for a status bar.
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.screen_size(0), Some(Region::new(0, 18, 800, 582)));
    /// assert_eq!(manager.screen_size(42), None);
    /// # Ok(())
    /// # }
    /// # example(example_windowmanager(2, vec![])).unwrap();
    /// ```
    pub fn screen_size(&self, screen_index: usize) -> Option<Region> {
        self.screens
            .get(screen_index)
            .map(|s| s.region(self.config.show_bar))
    }

    /// Position an individual client on the display. (x,y) coordinates are absolute (i.e. relative
    /// to the root window not any individual screen).
    pub fn position_client(&self, id: Xid, region: Region, stack_above: bool) -> Result<()> {
        let bpx = self.config.border_px;
        Ok(self.conn.position_client(id, region, bpx, stack_above)?)
    }

    /// Make the Client with ID 'id' visible at its last known position.
    pub fn show_client(&mut self, id: Xid) -> Result<()> {
        self.conn
            .map_client_if_needed(self.client_map.get_mut(&id))?;
        Ok(self.conn.set_client_workspace(id, self.active_ws_index())?)
    }

    /// Hide the Client with ID 'id'.
    pub fn hide_client(&mut self, id: Xid) -> Result<()> {
        Ok(self
            .conn
            .unmap_client_if_needed(self.client_map.get_mut(&id))?)
    }

    /// Layout the workspace currently shown on the given screen index.
    pub fn layout_screen(&mut self, screen_index: usize) -> Result<()> {
        if let Some(wix) = self.screens.get(screen_index).map(|s| s.wix) {
            self.apply_layout(wix)?;
        }

        Ok(())
    }

    /// An index into the WindowManager known screens for the screen that is currently focused
    ///
    /// # Example
    ///
    /// ```
    /// # use penrose::__example_helpers::*;
    /// # fn example(mut manager: ExampleWM) -> penrose::Result<()> {
    /// assert_eq!(manager.active_screen_index(), 0);
    ///
    /// manager.cycle_screen(Forward)?;
    /// assert_eq!(manager.active_screen_index(), 1);
    /// # Ok(())
    /// # }
    /// # let mut manager = example_windowmanager(2, n_clients(3));
    /// # manager.init().unwrap();
    /// # manager.grab_keys_and_run(example_key_bindings(), example_mouse_bindings()).unwrap();
    /// # example(manager).unwrap();
    /// ```
    pub fn active_screen_index(&self) -> usize {
        self.screens.focused_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        __example_helpers::{RecordedCall, RecordingXConn},
        core::{
            data_types::*,
            helpers::logging_error_handler,
            layout::*,
            ring::Direction::*,
            screen::*,
            xconnection::{MockXConn, XEvent},
        },
        draw::Color,
    };

    use std::{cell::Cell, convert::TryFrom};

    fn wm_with_mock_conn(events: Vec<XEvent>, unmanaged_ids: Vec<Xid>) -> WindowManager<MockXConn> {
        let conn = MockXConn::new(test_screens(), events, unmanaged_ids);
        let conf = Config {
            layouts: test_layouts(false),
            ..Default::default()
        };
        let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
        wm.init().unwrap();

        wm
    }

    fn test_layouts(follow_focus: bool) -> Vec<Layout> {
        let conf = LayoutConf {
            follow_focus,
            ..Default::default()
        };
        vec![Layout::new("t", conf, mock_layout, 1, 0.6)]
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
    fn killing_a_client_does_not_remove_it_from_the_workspace() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0);
        // Should trigger the kill but we wait for DestroyNotify before removing the
        // client state
        wm.kill_client().unwrap();

        assert_eq!(wm.workspaces[0].len(), 1);
    }

    #[test]
    fn client_to_workspace_inserts_at_head() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 2, 0); // [20, 10]
        wm.client_to_workspace(&Selector::Index(1)).unwrap(); // 20 -> ws::1
        wm.client_to_workspace(&Selector::Index(1)).unwrap(); // 10 -> ws::1, [10, 20]
        wm.focus_workspace(&Selector::Index(1)).unwrap();

        assert_eq!(
            wm.workspaces[1].iter().collect::<Vec<&Xid>>(),
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
        wm.update_focus(10).unwrap();

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

        wm.update_focus(20).unwrap();
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

    __impl_stub_xcon! {
        for ScreenChangingXConn;

        atom_queries: {}
        client_properties: {}
        client_handler: {}
        client_config: {}
        event_handler: {}
        state: {
            fn mock_current_screens(&self) -> crate::core::xconnection::Result<Vec<Screen>> {
                let num_screens = self.num_screens.get();
                Ok((0..(num_screens))
                    .map(|n| Screen::new(Region::new(800 * n as u32, 600 * n as u32, 800, 600), n))
                    .collect())
            }
        }
        conn: {}
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

    // Check that workspace layout is triggered correctly from public methods

    macro_rules! layout_trigger_test {
        { $method:ident; $should_layout:expr; $($arg:expr),* } => {
            paste::paste! {
                #[test]
                fn [<layout_trigger_test _ $method>]() {
                    let conn = RecordingXConn::init();
                    let conf = Config {
                        layouts: test_layouts(false),
                        ..Default::default()
                    };
                    let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
                    wm.init().unwrap();
                    add_n_clients(&mut wm, 3, 0);
                    wm.focus_workspace(&Selector::Index(1)).unwrap();
                    add_n_clients(&mut wm, 3, 30);
                    wm.focus_workspace(&Selector::Index(0)).unwrap();
                    wm.conn.clear();
                    wm.$method($($arg),*).unwrap();

                    // Defining "we applied layout" as "position_client" was called
                    // at least once. Tests around layout application itself being
                    // correct are handled separately
                    let did_layout = wm
                        .conn
                        .calls()
                        .iter()
                        .any(|c| c.0 == *"position_client");
                    assert_eq!(did_layout, $should_layout);
                }
            }
        }
    }

    layout_trigger_test!(cycle_workspace; true; Forward);
    layout_trigger_test!(drag_workspace; true; Forward);
    layout_trigger_test!(cycle_client; false; Forward);
    layout_trigger_test!(focus_client; false; &Selector::Any);
    layout_trigger_test!(rotate_clients; true; Forward);
    layout_trigger_test!(drag_client; true; Forward);
    layout_trigger_test!(cycle_layout; true; Forward);
    layout_trigger_test!(update_max_main; true; Change::More);
    layout_trigger_test!(update_main_ratio; true; Change::More);
    layout_trigger_test!(exit; false;);
    layout_trigger_test!(set_root_window_name; false; "test");
    layout_trigger_test!(set_client_insert_point; false; InsertPoint::First);
    layout_trigger_test!(focus_workspace; true; &Selector::Index(1));
    layout_trigger_test!(toggle_workspace; true;);
    layout_trigger_test!(client_to_workspace; true; &Selector::Index(1));
    layout_trigger_test!(client_to_screen; true; &Selector::Index(1));
    layout_trigger_test!(toggle_client_fullscreen; true; &Selector::WinId(10));
    layout_trigger_test!(kill_client; false;);
    layout_trigger_test!(remove_workspace; true; &Selector::Index(0));
    layout_trigger_test!(position_client; true; 10, Region::default(), true);
    layout_trigger_test!(layout_screen; true; 0);

    /*
     * Helpers for specifying expected events with RecordingXConn
     */

    fn _focus(id: Xid) -> RecordedCall {
        ("focus_client".into(), strings!(id))
    }

    fn _take_focus(id: Xid) -> RecordedCall {
        let conn = RecordingXConn::init();
        let evt = ClientMessageKind::TakeFocus(id).as_message(&conn).unwrap();
        ("send_client_event".into(), strings!(evt))
    }

    fn _id(a: Atom) -> RecordedCall {
        ("atom_id".into(), strings!(a.as_ref()))
    }

    fn _border(id: Xid, focused: bool) -> RecordedCall {
        let color = if focused {
            Color::try_from("#00ff00").unwrap()
        } else {
            Color::try_from("#ff0000").unwrap()
        };
        ("set_client_border_color".into(), strings!(id, color))
    }

    fn _active(id: Xid) -> RecordedCall {
        let args = strings!(42, "_NET_ACTIVE_WINDOW", Prop::Window(vec![id]));
        ("change_prop".into(), args)
    }

    fn _remove_active() -> RecordedCall {
        ("delete_prop".into(), strings!(42, "_NET_ACTIVE_WINDOW"))
    }

    test_cases! {
        update_focus;
        args: (
            target: Xid,
            accepts_focus: bool,
            current: Option<Xid>,
            n_clients: usize,
            follow_focus: bool,
            expected_focus: Option<Xid>,
            expected_calls: Vec<RecordedCall>
        );

        // We should still run focusing logic when the requested target is our current focus
        case: client_is_current_focus => (
            10, true, Some(10), 3, false,
            Some(10), vec![_focus(10), _active(10), _border(10, true)]
        );

        // We should remove the focused border from the current client first
        case: client_is_not_current_focus => (
            20, true, Some(10), 3, false,
            Some(20), vec![_border(10, false), _focus(20), _active(20), _border(20, true)]
        );

        // Focus should default to the focused client on the active workspace if the given client
        // is not in the client_map
        case: client_is_unknown_workspace_populated => (
            999, true, Some(10), 3, false,
            Some(30), vec![_border(10, false), _focus(30), _active(30), _border(30, true)]
        );

        // If the client is unknown and the workspace is empty, focus should revert to root
        case: client_is_unknown_workspace_empty => (
            999, true, None, 0, false,
            None, vec![_focus(42), _remove_active()]
        );

        // If the client doesn't accept focus then we should still mark it as focused in the
        // internal state, but a TakeFocus client message should be sent instead of forcing
        // focus.
        case: client_does_not_accept_focus_different => (
            20, false, Some(10), 3, false,
            Some(20), vec![
                _border(10, false), _id(Atom::WmTakeFocus), _take_focus(20)
            ]
        );

        // If the client doesn't accept focus, and it is the current focus then we should just
        // set the border and send the TakeFocus event
        case: client_does_not_accept_focus_same => (
            20, false, Some(20), 3, false,
            Some(20), vec![_id(Atom::WmTakeFocus), _take_focus(20)]
        );

        // TODO: add test cases for follow_focus layout triggering

        body: {
            let conn = RecordingXConn::init();
            let conf = Config {
                layouts: test_layouts(follow_focus),
                focused_border: Color::try_from("#00ff00").unwrap(),
                unfocused_border: Color::try_from("#ff0000").unwrap(),
                ..Default::default()
            };
            let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
            wm.init().unwrap();
            add_n_clients(&mut wm, n_clients, 0);
            wm.conn().clear();
            wm.focused_client = current;
            wm.client_map.entry(target).and_modify(|c| c.accepts_focus = accepts_focus);

            wm.update_focus(target).unwrap();

            assert_eq!(wm.focused_client, expected_focus);
            assert_eq!(wm.conn().calls(), expected_calls);
        }
    }
}
