//! The main user API and control logic for Penrose.
use crate::{
    core::{
        bindings::{KeyBindings, KeyCode, MouseBindings, MouseEvent},
        client::Client,
        config::Config,
        data_types::{Change, Point, Region},
        hooks::{HookName, Hooks},
        ring::{Direction, InsertPoint, Selector},
        screen::Screen,
        workspace::Workspace,
        xconnection::{Atom, ClientMessageKind, WindowState, XConn, Xid},
    },
    ErrorHandler, PenroseError, Result,
};
use nix::sys::signal::{signal, SigHandler, Signal};
use std::{cell::Cell, fmt};
use tracing::Level;

#[cfg(feature = "serde")]
use crate::core::{helpers::logging_error_handler, layout::LayoutFunc};

#[cfg(feature = "serde")]
use std::collections::HashMap;

mod clients;
mod event;
mod layout;
mod screens;
mod state;
mod util;
mod workspaces;

use clients::Clients;
use event::process_next_event;
use event::EventAction;
use layout::{apply_layout, layout_visible};
use screens::Screens;
use state::WmState;
use workspaces::Workspaces;

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
    pub(super) conn: X,
    pub(super) state: WmState,
    #[cfg_attr(feature = "serde", serde(skip, default = "default_hooks"))]
    pub(super) hooks: Cell<Hooks<X>>,
    pub(super) previous_workspace: usize,
    pub(super) running: bool,
    #[cfg_attr(feature = "serde", serde(skip, default = "logging_error_handler"))]
    pub(super) error_handler: ErrorHandler,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(super) hydrated: bool,
}

impl<X: XConn> fmt::Debug for WindowManager<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WindowManager")
            .field("conn", &stringify!(self.conn))
            .field("state", &self.state)
            .field("hooks", &stringify!(self.hooks))
            .field("previous_workspace", &self.previous_workspace)
            .field("running", &self.running)
            .finish()
    }
}

impl<X: XConn> WindowManager<X> {
    /// Construct a new window manager instance using a chosen [XConn] backed to communicate
    /// with the X server.
    pub fn new(config: Config, conn: X, hooks: Hooks<X>, error_handler: ErrorHandler) -> Self {
        let layouts = config.layouts.clone();

        trace!("building initial workspaces");
        let workspaces = Workspaces::new(
            config
                .workspaces
                .iter()
                .map(|name| Workspace::new(name, layouts.to_vec()))
                .collect(),
            config.main_ratio_step,
        );

        let screens = Screens::new(config.bar_height, config.top_bar);
        let clients = Clients::new(config.focused_border, config.unfocused_border);

        let state = WmState {
            config,
            clients,
            screens,
            workspaces,
        };

        Self {
            conn,
            state,
            previous_workspace: 0,
            hooks: Cell::new(hooks),
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
        self.workspaces.restore_layout_functions(&layout_funcs)?;
        util::validate_hydrated_wm_state(self)?;
        self.hydrated = true;
        self.init()?;
        Ok(())
    }

    /// This initialises the [WindowManager] internal state but does not start processing any
    /// events from the X server. If you need to perform any custom setup logic with the
    /// [WindowManager] itself, it should be run after calling this method and before
    /// [WindowManager::grab_keys_and_run].
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
        Ok(self.conn.warp_cursor(None, &self.screens.inner[0])?)
    }

    #[tracing::instrument(level = "debug", err, skip(self))]
    pub(crate) fn try_manage_existing_windows(&mut self) -> Result<()> {
        let classes = str_slice!(self.config.floating_classes);
        for mut c in self.conn.active_managed_clients(classes)?.into_iter() {
            let id = c.id();
            self.add_client_to_workspace(c.workspace(), id)?;
            self.conn.unmap_client_if_needed(Some(&mut c))?;
            self.clients.insert(id, c);
            self.conn.mark_new_client(id)?;
        }

        if let Some(id) = self.workspaces.focused_client(0) {
            self.update_focus(id)?;
        }

        self.update_known_x_clients()?;
        self.layout_visible()?;
        Ok(())
    }

    #[tracing::instrument(level = "trace", skip(self))]
    fn run_hook(&mut self, hook_name: HookName) {
        use HookName::*;

        // Relies on all hooks taking &mut WindowManager as the first arg.
        macro_rules! run_hooks {
            ($method:ident, $_self:expr, $($arg:expr),*) => {
                {
                    debug!(target: "hooks", "Running {} hooks", stringify!($method));
                    let mut hooks = $_self.hooks.replace(vec![]);
                    let res = hooks.iter_mut().try_for_each(|h| h.$method($_self, $($arg),*));
                    $_self.hooks.replace(hooks);
                    if let Err(e) = res {
                        ($_self.error_handler)(e);
                    }
                }
            };
        }

        match hook_name {
            Startup => run_hooks!(startup, self,),
            NewClient(id) => run_hooks!(new_client, self, id),
            RemoveClient(id) => run_hooks!(remove_client, self, id),
            ClientAddedToWorkspace(id, wix) => run_hooks!(client_added_to_workspace, self, id, wix),
            ClientNameUpdated(id, name, is_root) => {
                run_hooks!(client_name_updated, self, id, &name, is_root);
            }
            LayoutApplied(wix, i) => run_hooks!(layout_applied, self, wix, i),
            LayoutChange(wix) => {
                let i = self.active_screen_index();
                run_hooks!(layout_change, self, wix, i);
            }
            WorkspaceChange(active, index) => run_hooks!(workspace_change, self, active, index),
            WorkspacesUpdated(names, wix) => {
                run_hooks!(workspaces_updated, self, str_slice!(names), wix)
            }
            ScreenChange => {
                let i = self.screens.focused_index();
                run_hooks!(screen_change, self, i);
            }
            ScreenUpdated => {
                let regions = self.screens.inner.vec_map(|s| s.region(false));
                run_hooks!(screens_updated, self, &regions);
            }
            RanderNotify => run_hooks!(randr_notify, self,),
            FocusChange(root) => run_hooks!(focus_change, self, root),
            EventHandled => run_hooks!(event_handled, self,),
        }
    }

    fn handle_event_actions(&mut self, actions: Vec<EventAction>) -> Result<()> {
        for a in actions {
            self.handle_event_action(a, None, None)?;
        }

        Ok(())
    }

    // Each XEvent from the XConn can result in multiple EventActions that need processing
    // depending on the current WindowManager state.
    #[tracing::instrument(level = "trace", err, skip(self, key_bindings, mouse_bindings))]
    fn handle_event_action(
        &mut self,
        action: EventAction,
        key_bindings: Option<&mut KeyBindings<X>>,
        mouse_bindings: Option<&mut MouseBindings<X>>,
    ) -> Result<()> {
        use EventAction::*;

        match action {
            ClientFocusGained(id) => self.update_focus(id)?,
            ClientFocusLost(id) => self.state.clients.client_lost_focus(id, &self.conn),
            ClientNameChanged(id, is_root) => {
                let action = self
                    .state
                    .clients
                    .client_name_changed(id, is_root, &self.conn)?;
                self.handle_event_action(action, None, None)?
            }
            ClientToWorkspace(id, wix) => self.move_client_to_workspace(id, wix)?,
            DestroyClient(id) => self.remove_client(id)?,
            DetectScreens => {
                self.run_hook(HookName::RanderNotify);
                self.detect_screens()?
            }
            FocusIn(id) => self.clients.focus_in(id, &self.conn)?,
            LayoutVisible => self.layout_visible()?,
            LayoutWorkspace(wix) => self.apply_layout(wix)?,
            MapWindow(id) => self.handle_map_request(id)?,
            MoveClientIfFloating(id, r) => self.handle_move_if_floating(id, r)?,
            RunHook(hook_name) => self.run_hook(hook_name),
            RunKeyBinding(e) => match key_bindings {
                Some(kb) => self.run_key_binding(e, kb),
                None => return Err(perror!("keybindings can only be triggered from X events")),
            },
            RunMouseBinding(e) => match mouse_bindings {
                Some(mb) => self.run_mouse_binding(e, mb),
                None => return Err(perror!("mousebindings can only be triggered from X events")),
            },
            SetActiveClient(id) => self.set_active_client(id)?,
            SetActiveWorkspace(wix) => self.focus_workspace(&Selector::Index(wix))?,
            SetScreenFromPoint(p) => self.set_screen_from_point(p)?,
            ToggleClientFullScreen(id, should_fullscreen) => {
                self.set_fullscreen(id, should_fullscreen)?;
            }
            UnknownPropertyChange(id, atom, is_root) => {
                self.handle_prop_change(id, atom, is_root)?;
            }
            Unmap(id) => self.handle_unmap_notify(id)?,
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

        self.run_hook(HookName::Startup);
        self.running = true;

        trace!("entering main event loop");
        while self.running {
            match self.conn.wait_for_event() {
                Ok(event) => {
                    let span = span!(target: "penrose", Level::DEBUG, "XEvent", %event);
                    let _enter = span.enter();
                    trace!(details = ?event, "event details");

                    let actions = process_next_event(event, &self.state, &self.conn);
                    for action in actions {
                        if let Err(e) = self.handle_event_action(
                            action,
                            Some(&mut key_bindings),
                            Some(&mut mouse_bindings),
                        ) {
                            (self.error_handler)(e);
                        }
                    }

                    self.run_hook(HookName::EventHandled);
                    self.conn.flush();
                }

                Err(e) => (self.error_handler)(PenroseError::X(e)),
            }
        }

        Ok(())
    }

    /*
     * Top Level EventAction handlers
     */

    // Set the current focus point based on client focus hints
    #[tracing::instrument(level = "trace", err, skip(self))]
    fn update_focus(&mut self, id: Xid) -> Result<()> {
        let target = if self.clients.is_known(id) {
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
                    self.run_hook(HookName::FocusChange(root));
                    return Ok(());
                }
            }
        };

        let prev = self.state.clients.set_focused(target, &self.conn);

        let (wix, accepts_focus) = {
            // Safe to unwrap because we make sure this is a known client above
            let c = self.clients.get(target).unwrap();
            (c.workspace(), c.accepts_focus)
        };

        self.focus_screen(&Selector::Condition(&|s| s.wix == wix));
        self.clients
            .set_x_focus(target, accepts_focus, &self.conn)?;

        if let Some(ws) = self.workspaces.get_mut(wix) {
            ws.focus_client(target);
            let in_ws = prev.map_or(false, |prev_id| ws.client_ids().contains(&prev_id));
            if ws.layout_conf().follow_focus && in_ws {
                if let Err(e) = self.apply_layout(wix) {
                    error!("unable to apply layout on ws {}: {}", wix, e);
                }
            }
        }

        self.run_hook(HookName::FocusChange(target));
        Ok(())
    }

    // The given window ID has been destroyed so remove our internal state referencing it.
    #[tracing::instrument(level = "trace", err, skip(self))]
    fn remove_client(&mut self, id: Xid) -> Result<()> {
        if let Some(client) = self.clients.remove(id) {
            let wix = client.workspace();
            self.workspaces.remove_client(wix, id);

            if self.screens.visible_workspaces().contains(&wix) {
                self.apply_layout(wix)?;
            }

            self.update_known_x_clients()?;
            self.run_hook(HookName::RemoveClient(id));
        } else {
            debug!(id, "attempt to remove unknown client");
        }

        Ok(())
    }

    #[tracing::instrument(level = "trace", err, skip(self))]
    fn move_client_to_workspace(&mut self, id: Xid, wix: usize) -> Result<()> {
        let current_wix = match self.clients.workspace_index_for_client(id) {
            Some(ix) => ix,
            None => return Err(PenroseError::UnknownClient(id)),
        };

        if current_wix != wix {
            self.workspaces.remove_client(current_wix, id);
            self.add_client_to_workspace(wix, id)?;
            self.clients.set_client_workspace(id, wix);

            if self.screens.visible_workspaces().contains(&wix) {
                let s = self.screens.focused();
                self.conn.warp_cursor(Some(id), s)?;
            } else {
                self.state.clients.unmap_if_needed(id, &self.conn)?;
            }

            self.layout_visible()?;
        }

        Ok(())
    }

    /// Query the [XConn] for the current connected [Screen] list and reposition displayed
    /// [Workspace] instances if needed.
    #[tracing::instrument(level = "trace", err, skip(self))]
    pub fn detect_screens(&mut self) -> Result<()> {
        let actions = self
            .state
            .screens
            .update_known_screens(&self.conn, self.workspaces.len())?;

        self.handle_event_actions(actions)
    }

    // Map a new client window.
    #[tracing::instrument(level = "trace", err, skip(self))]
    fn handle_map_request(&mut self, id: Xid) -> Result<()> {
        trace!(id, "handling map request");
        let classes = str_slice!(self.config.floating_classes);
        let client = Client::new(&self.conn, id, self.screens.active_ws_index(), classes);
        let is_managed_type = self.conn.is_managed_client(&client);
        trace!(id, ?client.wm_name, ?client.wm_class, ?client.wm_type, "client details");

        // Run hooks to allow them to modify the client
        self.clients.insert(id, client);
        self.run_hook(HookName::NewClient(id));

        let details = self
            .clients
            .get(id)
            .map(|c| (c.workspace(), c.wm_hints.clone(), c.wm_managed, c.floating));

        if details.is_none() {
            debug!(id, "Client was removed from the client map by a hook");
            return Ok(());
        }

        let (wix, wm_hints, wm_managed, floating) = details.unwrap();

        if let Some(ref wmh) = wm_hints {
            if wmh.initial_state == WindowState::Withdrawn {
                self.clients.remove(id);
                return Ok(()); // Don't map withdrawn clients
            }
        }

        if !is_managed_type {
            return Ok(self.conn.map_client(id)?);
        }

        if wm_managed {
            self.add_client_to_workspace(wix, id)?;
        }

        if floating {
            if let Some((_, s)) = self.screens.indexed_screen_for_workspace(wix) {
                util::position_floating_client(
                    &self.conn,
                    id,
                    s.region(self.config.show_bar),
                    self.config.border_px,
                )?
            }
        }

        self.conn.mark_new_client(id)?;
        self.update_focus(id)?;
        self.update_known_x_clients()?;

        if wix == self.screens.active_ws_index() {
            self.apply_layout(wix)?;
            self.state.clients.map_if_needed(id, &self.conn)?;
            let s = self.screens.focused();
            self.conn.warp_cursor(Some(id), s)?;
        }

        Ok(())
    }

    fn handle_move_if_floating(&mut self, id: Xid, r: Region) -> Result<()> {
        if let Some(client) = self.clients.get(id) {
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
            .map_err(|_| PenroseError::UnknownClient(id))
            .map(|_| ())
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
            .clients
            .get(id)
            .map(|c| (c.fullscreen, c.workspace()))
            .ok_or(PenroseError::UnknownClient(id))?;

        if currently_fullscreen == should_fullscreen {
            return Ok(()); // Client is already in the correct state, we shouldn't have been called
        }

        let r = match self.screen(&Selector::Condition(&|s| s.wix == wix)) {
            Some(s) => s.region(false),
            None => return Ok(()),
        };

        let client_ids = self.workspaces.client_ids(wix)?;
        let actions = self
            .state
            .clients
            .toggle_fullscreen(id, wix, &client_ids, r, &self.conn)?;

        self.handle_event_actions(actions)
    }

    /*
     * Common mid level actions that make up larger event response handlers.
     */

    fn apply_layout(&mut self, wix: usize) -> Result<()> {
        if let Some(action) = apply_layout(&mut self.state, &self.conn, wix)? {
            self.handle_event_action(action, None, None)
        } else {
            Ok(())
        }
    }

    #[tracing::instrument(level = "trace", err, skip(self))]
    fn layout_visible(&mut self) -> Result<()> {
        let actions = layout_visible(&mut self.state, &self.conn)?;
        self.handle_event_actions(actions)
    }

    fn update_x_workspace_details(&mut self) -> Result<()> {
        let names = self.workspaces.workspace_names();
        self.conn.update_desktops(&names)?;
        self.run_hook(HookName::WorkspacesUpdated(
            names,
            self.screens.active_ws_index(),
        ));

        Ok(())
    }

    fn update_known_x_clients(&self) -> Result<()> {
        let ids = self.clients.all_known_ids();
        Ok(self.conn.update_known_clients(&ids)?)
    }

    fn focus_screen(&mut self, sel: &Selector<'_, Screen>) -> &Screen {
        let actions = self.screens.focus_screen(sel);
        if let Err(e) = self.handle_event_actions(actions) {
            (self.error_handler)(e);
        }

        let wix = self.screens.focused().wix;
        if let Err(e) = self.conn.set_current_workspace(wix) {
            error!("Got error when setting current workspace {}", e);
        };

        self.screens.focused()
    }

    #[tracing::instrument(level = "trace", err, skip(self))]
    fn add_client_to_workspace(&mut self, wix: usize, id: Xid) -> Result<()> {
        self.clients.modify(id, |c| c.set_workspace(wix));
        if let Some(action) = self.workspaces.add_client(wix, id)? {
            self.conn.set_client_workspace(id, wix)?;
            self.handle_event_action(action, None, None)?;
        }

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
    pub fn conn_mut(&mut self) -> &mut X {
        &mut self.conn
    }

    /// The currently focused client ID if there is one
    pub fn focused_client_id(&self) -> Option<Xid> {
        self.clients.focused_client_id()
    }

    /// Cycle between known [screens][Screen]. Does not wrap from first to last
    pub fn cycle_screen(&mut self, direction: Direction) -> Result<()> {
        let actions = self.state.screens.cycle_screen(direction, &self.conn)?;
        self.handle_event_actions(actions)
    }

    /// Cycle between [workspaces][1] on the current [screen][2].
    ///
    /// This method will pull workspaces to the active screen if they are currently displayed on
    /// another screen.
    ///
    /// [1]: Workspace
    /// [2]: Screen
    pub fn cycle_workspace(&mut self, direction: Direction) -> Result<()> {
        let i = self.workspaces.cycle_workspace(direction);
        self.focus_workspace(&Selector::Index(i))
    }

    /// Move the currently focused [Workspace] to the next [Screen] in 'direction'
    pub fn drag_workspace(&mut self, direction: Direction) -> Result<()> {
        let wix = self.screens.active_ws_index();
        self.cycle_screen(direction)?;
        self.focus_workspace(&Selector::Index(wix)) // focus_workspace will pull it to the new screen
    }

    /// Cycle focus between [clients][1] for the active [Workspace]
    ///
    /// [1]: Client
    pub fn cycle_client(&mut self, direction: Direction) -> Result<()> {
        let wix = self.screens.active_ws_index();
        let res = self.workspaces.cycle_client(wix, direction);
        if let Some((prev, new)) = res {
            self.state.clients.client_lost_focus(prev, &self.conn);
            self.update_focus(new)?;
            let screen = self.screens.focused();
            self.conn.warp_cursor(Some(new), screen)?;
        }

        Ok(())
    }

    /// Focus the [Client] matching the given [Selector]
    pub fn focus_client(&mut self, selector: &Selector<'_, Client>) -> Result<Xid> {
        let id = match self.client(selector) {
            Some(c) => c.id(),
            None => return Err(PenroseError::NoMatchingElement),
        };
        self.update_focus(id)?;
        let screen = self.screens.focused();
        self.conn.warp_cursor(Some(id), screen)?;
        Ok(id)
    }

    /// Rotate the [Client] stack on the active [Workspace].
    ///
    /// This maintains the current window layout but permutes the positions of each window within
    /// that layout.
    pub fn rotate_clients(&mut self, direction: Direction) -> Result<()> {
        let wix = self.screens.active_ws_index();
        self.workspaces.rotate_clients(wix, direction);
        self.apply_layout(wix)
    }

    /// Move the focused [Client] through the stack of clients on the active [Workspace].
    pub fn drag_client(&mut self, direction: Direction) -> Result<()> {
        if let Some(id) = self.clients.focused_client_id() {
            let wix = self.screens.active_ws_index();
            self.workspaces.drag_client(wix, direction);
            self.apply_layout(wix)?;
            self.update_focus(id)?;
            self.conn.warp_cursor(Some(id), self.screens.focused())?;
        }

        Ok(())
    }

    /// Cycle between [layouts][1] for the active [Workspace]
    ///
    /// [1]: crate::core::layout::Layout
    pub fn cycle_layout(&mut self, direction: Direction) -> Result<()> {
        let wix = self.screens.active_ws_index();
        self.workspaces.cycle_layout(wix, direction);
        self.run_hook(HookName::LayoutChange(wix));
        self.apply_layout(wix)
    }

    /// Increase or decrease the number of clients in the main area by 1.
    ///
    /// The change is applied to the active [layout][1] on the [Workspace] that currently holds
    /// focus.
    ///
    /// [1]: crate::core::layout::Layout
    pub fn update_max_main(&mut self, change: Change) -> Result<()> {
        let wix = self.screens.active_ws_index();
        self.workspaces.update_max_main(wix, change);
        self.apply_layout(wix)
    }

    /// Increase or decrease the current [layout][1] main_ratio by `main_ratio_step`
    ///
    /// The change is applied to the active [layout][1] on the [Workspace] that currently holds
    /// focus.
    ///
    /// [1]: crate::core::layout::Layout
    pub fn update_main_ratio(&mut self, change: Change) -> Result<()> {
        let wix = self.screens.active_ws_index();
        self.workspaces.update_main_ratio(wix, change);
        self.apply_layout(wix)
    }

    /// Shut down the WindowManager, running any required cleanup and exiting penrose
    ///
    /// **NOTE**: any registered hooks on the `WindowManager` will still run following calling this
    /// method, with the actual exit condition being checked and handled at the end.
    pub fn exit(&mut self) -> Result<()> {
        self.conn.cleanup()?;
        self.conn.flush();
        self.running = false;

        Ok(())
    }

    /// The layout symbol for the [layout][1] currently being used on the
    /// active workspace
    ///
    /// [1]: crate::core::layout::Layout
    pub fn current_layout_symbol(&self) -> &str {
        let wix = self.screens.active_ws_index();
        self.workspaces.current_layout_symbol(wix)
    }

    /// Set the root X window name. Useful for exposing information to external programs
    pub fn set_root_window_name(&self, s: impl AsRef<str>) -> Result<()> {
        Ok(self.conn.set_root_window_name(s.as_ref())?)
    }

    /// Set the insert point for new clients. Default is to insert at index 0.
    pub fn set_client_insert_point(&mut self, cip: InsertPoint) -> Result<()> {
        self.workspaces.set_client_insert_point(cip);

        Ok(())
    }

    /// Set the displayed workspace for the focused screen to be `index` in the list of
    /// workspaces passed at `init`.
    ///
    /// A common way to use this method is in a `refMap` section when generating your keybindings
    /// and using the [index_selectors][1] helper method to make the required [selectors][2].
    ///
    /// [1]: crate::core::helpers::index_selectors
    /// [2]: crate::core::ring::Selector
    pub fn focus_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Result<()> {
        let ix = self.screens.focused().wix;
        if self.workspaces.would_focus(ix, selector) {
            return Ok(());
        }

        if let Some(index) = self.workspaces.index(selector) {
            let active = self.screens.active_ws_index();
            self.previous_workspace = active;

            for i in 0..self.screens.n_screens() {
                if self.screens.inner[i].wix == index {
                    // The workspace we want is currently displayed on another screen so
                    // pull the target workspace to the focused screen, and place the
                    // workspace we had on the screen where the target was
                    self.screens.inner[i].wix = self.screens.focused().wix;
                    self.screens.focused_mut().wix = index;

                    // re-apply layouts as screen dimensions may differ
                    self.apply_layout(active)?;
                    self.apply_layout(index)?;

                    // update xproperty _NET_CURRENT_DESKTOP
                    self.conn.set_current_workspace(index)?;

                    let ws = self.workspaces.get_workspace(index)?;
                    if let Some(id) = ws.focused_client() {
                        self.update_focus(id)?;
                    };

                    self.workspaces.focus(&Selector::Index(index));
                    self.run_hook(HookName::WorkspaceChange(active, index));
                    return Ok(());
                }
            }

            // target not currently displayed so unmap what we currently have
            // displayed and replace it with the target workspace
            let ws = self.workspaces.get_workspace(active)?;
            for id in ws.client_ids().iter() {
                self.state.clients.unmap_if_needed(*id, &self.conn)?;
            }

            let ws = self.workspaces.get_workspace(index)?;
            for id in ws.client_ids().iter() {
                self.state.clients.map_if_needed(*id, &self.conn)?;
            }

            self.screens.focused_mut().wix = index;
            self.apply_layout(index)?;
            self.conn.set_current_workspace(index)?;

            let ws = self.workspaces.get_workspace(index)?;
            if let Some(id) = ws.focused_client() {
                self.update_focus(id)?;
            };

            self.workspaces.focus(&Selector::Index(index));
            self.run_hook(HookName::WorkspaceChange(active, index));
        }

        Ok(())
    }

    /// Switch focus back to the last workspace that had focus.
    pub fn toggle_workspace(&mut self) -> Result<()> {
        self.focus_workspace(&Selector::Index(self.previous_workspace))
    }

    /// Move the focused client to the workspace matching 'selector'.
    pub fn client_to_workspace(&mut self, selector: &Selector<'_, Workspace>) -> Result<()> {
        if let Some(id) = self.clients.focused_client_id() {
            if let Some(wix) = self.workspaces.index(selector) {
                self.move_client_to_workspace(id, wix)?;
                if let Some(now_focused) = self.active_workspace().focused_client() {
                    self.state.clients.set_focused(now_focused, &self.conn);
                }
            }
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

    /// Toggle the fullscreen state of the [Client] matching the given [Selector]
    pub fn toggle_client_fullscreen(&mut self, selector: &Selector<'_, Client>) -> Result<()> {
        let (id, client_is_fullscreen) = match self.client(selector) {
            None => return Ok(()), // unknown client
            Some(c) => (c.id(), c.fullscreen),
        };
        self.set_fullscreen(id, !client_is_fullscreen)
    }

    /// Kill the focused client window.
    #[tracing::instrument(level = "debug", err, skip(self))]
    pub fn kill_client(&mut self) -> Result<()> {
        if let Some(id) = self.clients.focused_client_id() {
            let msg = ClientMessageKind::DeleteWindow(id).as_message(&self.conn)?;
            self.conn.send_client_event(msg)?;
            self.conn.flush();
        }

        Ok(())
    }

    /// Get a reference to the first Screen satisfying 'selector'. Xid selectors will return
    /// the screen containing that Client if the client is known.
    /// NOTE: It is not possible to get a mutable reference to a Screen.
    pub fn screen(&self, selector: &Selector<'_, Screen>) -> Option<&Screen> {
        if let Selector::WinId(id) = selector {
            let wix = self.clients.workspace_index_for_client(*id)?;
            self.screens.screen(&Selector::Condition(&|s| s.wix == wix))
        } else {
            self.screens.screen(&selector)
        }
    }

    /// An immutable reference to the current active [Workspace]
    pub fn active_workspace(&self) -> &Workspace {
        self.workspaces
            .workspace(&Selector::Index(self.screens.active_ws_index()))
            .expect("no active workspace")
    }

    /// A mutable reference to the current active [Workspace]
    pub fn active_workspace_mut(&mut self) -> &mut Workspace {
        self.state
            .workspaces
            .workspace_mut(&Selector::Index(self.screens.active_ws_index()))
            .expect("no active workspace")
    }

    /// The currently focused workspace indices being shown on each screen
    pub fn focused_workspaces(&self) -> Vec<usize> {
        self.screens.visible_workspaces()
    }

    /// Add a new workspace at `index`, shifting all workspaces with indices greater to the right.
    pub fn add_workspace(&mut self, index: usize, ws: Workspace) -> Result<()> {
        self.workspaces.add_workspace(index, ws);
        self.update_x_workspace_details()
    }

    /// Add a new workspace at the end of the current workspace list
    pub fn push_workspace(&mut self, ws: Workspace) -> Result<()> {
        self.workspaces.push_workspace(ws);
        self.update_x_workspace_details()
    }

    /// Remove a Workspace from the WindowManager. All clients that were present on the removed
    /// workspace will be destroyed. Xid selectors will be ignored.
    pub fn remove_workspace(
        &mut self,
        selector: &Selector<'_, Workspace>,
    ) -> Result<Option<Workspace>> {
        if self.workspaces.len() == self.screens.n_screens() {
            return Err(perror!("must have at least one workspace per screen"));
        }

        let ws = self.workspaces.remove_workspace(&selector)?;
        ws.iter().try_for_each(|c| self.remove_client(*c))?;

        // Focus the workspace before the one we just removed. There is always at least one
        // workspace before this one due to the guard above.
        let ix = self.screens.focused().wix.saturating_sub(1);
        self.focus_workspace(&Selector::Index(ix))?;
        self.update_x_workspace_details()?;

        Ok(Some(ws))
    }

    /// Get a reference to the first Workspace satisfying 'selector'. Xid selectors will return
    /// the workspace containing that Client if the client is known.
    pub fn workspace(&self, selector: &Selector<'_, Workspace>) -> Option<&Workspace> {
        self.workspaces.workspace(selector)
    }

    /// Get a mutable reference to the first Workspace satisfying 'selector'. Xid selectors will
    /// return the workspace containing that Client if the client is known.
    pub fn workspace_mut(&mut self, selector: &Selector<'_, Workspace>) -> Option<&mut Workspace> {
        self.workspaces.workspace_mut(selector)
    }

    /// Get a vector of immutable references to _all_ workspaces that match the provided [Selector].
    pub fn all_workspaces(&self, selector: &Selector<'_, Workspace>) -> Vec<&Workspace> {
        self.workspaces.matching_workspaces(selector)
    }

    /// Get a vector of mutable references to _all_ workspaces that match the provided [Selector].
    pub fn all_workspaces_mut(
        &mut self,
        selector: &Selector<'_, Workspace>,
    ) -> Vec<&mut Workspace> {
        self.workspaces.matching_workspaces_mut(selector)
    }

    /// Set the name of the selected Workspace
    pub fn set_workspace_name(
        &mut self,
        name: impl Into<String>,
        selector: &Selector<'_, Workspace>,
    ) -> Result<()> {
        self.workspaces.set_workspace_name(name, selector);
        self.update_x_workspace_details()
    }

    /// Take a reference to the first Client found matching 'selector'
    pub fn client(&self, selector: &Selector<'_, Client>) -> Option<&Client> {
        match selector {
            Selector::Index(i) => self
                .workspaces
                .get(self.screens.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
                .and_then(|id| self.clients.get(*id)),
            _ => self.clients.client(selector),
        }
    }

    /// Take a mutable reference to the first Client found matching 'selector'
    pub fn client_mut(&mut self, selector: &Selector<'_, Client>) -> Option<&mut Client> {
        match selector {
            Selector::Index(i) => match self
                .state
                .workspaces
                .get(self.screens.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
            {
                Some(id) => self.state.clients.get_mut(*id),
                None => None,
            },
            _ => self.clients.client_mut(selector),
        }
    }

    /// Get a vector of references to the Clients found matching 'selector'.
    /// The resulting vector is sorted by Client id.
    pub fn all_clients(&self, selector: &Selector<'_, Client>) -> Vec<&Client> {
        let mut clients: Vec<&Client> = match selector {
            Selector::Index(i) => self
                .workspaces
                .get(self.screens.active_ws_index())
                .and_then(|ws| ws.iter().nth(*i))
                .and_then(|id| self.clients.get(*id))
                .into_iter()
                .collect(),
            _ => self.clients.matching_clients(selector),
        };

        clients.sort_unstable_by_key(|c| c.id());
        clients
    }

    /// Get a vector of mutable references to the Clients found matching 'selector'.
    ///
    /// The resulting vector is sorted by Client id.
    pub fn all_clients_mut(&mut self, selector: &Selector<'_, Client>) -> Vec<&mut Client> {
        let mut clients: Vec<&mut Client> = match selector {
            Selector::Index(i) => {
                match self
                    .state
                    .workspaces
                    .get(self.screens.active_ws_index())
                    .and_then(|ws| ws.iter().nth(*i))
                {
                    Some(id) => self.state.clients.get_mut(*id).into_iter().collect(),
                    None => vec![],
                }
            }
            _ => self.clients.matching_clients_mut(selector),
        };

        clients.sort_unstable_by_key(|c| c.id());
        clients
    }

    /// The number of detected screens currently being tracked by the WindowManager.
    pub fn n_screens(&self) -> usize {
        self.screens.n_screens()
    }

    /// The current effective screen size of the target screen. Effective screen size is the
    /// physical screen size minus any space reserved for a status bar.
    pub fn screen_size(&self, index: usize) -> Option<Region> {
        self.screens.screen_size(index, self.config.show_bar)
    }

    /// Position an individual client on the display. (x,y) coordinates are absolute (i.e. relative
    /// to the root window not any individual screen).
    pub fn position_client(&self, id: Xid, region: Region, stack_above: bool) -> Result<()> {
        let bpx = self.config.border_px;
        self.conn
            .position_client(id, region, bpx, stack_above)
            .map_err(|e| e.into())
    }

    /// Make the Client with ID 'id' visible at its last known position.
    pub fn show_client(&mut self, id: Xid) -> Result<()> {
        self.state.clients.map_if_needed(id, &self.conn)?;
        // TODO: is this right? Need to double check...
        self.conn
            .set_client_workspace(id, self.screens.active_ws_index())
            .map_err(|e| e.into())
    }

    /// Hide the Client with ID 'id'.
    pub fn hide_client(&mut self, id: Xid) -> Result<()> {
        self.state.clients.unmap_if_needed(id, &self.conn)
    }

    /// Layout the workspace currently shown on the given screen index.
    pub fn layout_screen(&mut self, screen_index: usize) -> Result<()> {
        if let Some(wix) = self.screens.get(screen_index).map(|s| s.wix) {
            self.apply_layout(wix)?;
        }

        Ok(())
    }

    /// An index into the WindowManager known screens for the screen that is currently focused
    pub fn active_screen_index(&self) -> usize {
        self.screens.active_screen_index()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        __test_helpers::{
            n_clients, test_key_bindings, test_mouse_bindings, test_windowmanager, RecordedCall,
            RecordingXConn,
        },
        core::{
            data_types::*,
            helpers::logging_error_handler,
            layout::*,
            ring::Direction::*,
            screen::*,
            xconnection::{MockXConn, Prop, XEvent},
        },
        draw::Color,
    };

    use std::{cell::Cell, collections::HashMap, convert::TryFrom};

    fn wm_with_mock_conn(events: Vec<XEvent>, unmanaged_ids: Vec<Xid>) -> WindowManager<MockXConn> {
        let conn = MockXConn::new(test_screens(), events, unmanaged_ids);
        let conf = Config {
            layouts: focus_test_layouts(false),
            ..Default::default()
        };
        let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
        wm.init().unwrap();

        wm
    }

    fn focus_test_layouts(follow_focus: bool) -> Vec<Layout> {
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

        assert_eq!(wm.clients.workspace_index_for_client(10), Some(0));
        wm.client_to_workspace(&Selector::Index(42)).unwrap();
        assert_eq!(wm.clients.workspace_index_for_client(10), Some(0));
    }

    #[test]
    fn client_to_screen_sets_correct_workspace() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        wm.client_to_screen(&Selector::Index(1)).unwrap();
        assert_eq!(wm.clients.workspace_index_for_client(10), Some(1));
    }

    #[test]
    fn client_to_invalid_screen_is_noop() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0); // [20, 10]

        assert_eq!(wm.clients.workspace_index_for_client(10), Some(0));
        wm.client_to_screen(&Selector::Index(5)).unwrap();
        assert_eq!(wm.clients.workspace_index_for_client(10), Some(0));
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
        assert_eq!(wm.workspaces.focused_index(), wm.screens.active_ws_index());
        wm.focus_workspace(&Selector::Index(3)).unwrap();
        assert_eq!(wm.workspaces.focused_index(), 3);
        assert_eq!(wm.workspaces.focused_index(), wm.screens.active_ws_index());
    }

    #[test]
    fn dragging_clients_forward_from_index_0() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 5, 0); // focus on last client (50) ix == 0

        let clients = |w: &mut WindowManager<_>| {
            w.workspaces[w.screens.get(0).unwrap().wix]
                .iter()
                .cloned()
                .collect::<Vec<_>>()
        };

        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.clients.focused_client_id(), Some(50));
        assert_eq!(clients(&mut wm), vec![40, 50, 30, 20, 10]);

        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.clients.focused_client_id(), Some(50));
        assert_eq!(clients(&mut wm), vec![40, 30, 50, 20, 10]);

        wm.update_focus(20).unwrap();
        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.clients.focused_client_id(), Some(20));
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
    fn selector_screen() {
        let mut wm = wm_with_mock_conn(vec![], vec![]);
        add_n_clients(&mut wm, 1, 0);

        assert_eq!(wm.screen(&Selector::Focused), Some(wm.screens.focused()));
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

        assert_eq!(wm.client(&Selector::Focused), wm.clients.get(40));
        assert_eq!(wm.client(&Selector::Index(2)), wm.clients.get(20));
        assert_eq!(wm.client(&Selector::WinId(30)), wm.clients.get(30));
        assert_eq!(
            wm.client(&Selector::Condition(&|c| c.id() == 10)),
            wm.clients.get(10)
        );
    }

    #[test]
    fn unmanaged_window_types_are_not_added_to_workspaces() {
        // Setting the unmanaged window IDs here sets the return of
        // MockXConn.is_managed_window to false for those IDs
        let mut wm = wm_with_mock_conn(vec![], vec![10]);

        wm.handle_map_request(10).unwrap(); // should not be tiled
        assert!(wm.clients.get(10).is_some());
        assert!(wm.workspaces[0].is_empty());

        wm.handle_map_request(20).unwrap(); // should be tiled
        assert!(wm.clients.get(20).is_some());
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

    // TODO: rewrite and move out to screens.rs
    #[test]
    fn updating_screens_retains_focused_workspaces() {
        let conn = ScreenChangingXConn {
            num_screens: Cell::new(1),
        };
        let conf = Config::default();
        let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
        wm.init().unwrap();

        // detect_screens is called on init so should have one screen
        assert_eq!(wm.screens.n_screens(), 1);
        assert_eq!(wm.screens.focused().wix, 0);

        // Focus workspace 1 the redetect screens: should have 1 and 0
        wm.focus_workspace(&Selector::Index(1)).unwrap();
        assert_eq!(wm.screens.focused().wix, 1);
        wm.set_num_screens(2);
        wm.detect_screens().unwrap();
        assert_eq!(wm.screens.n_screens(), 2);
        assert_eq!(wm.screens.get(0).unwrap().wix, 1);
        assert_eq!(wm.screens.get(1).unwrap().wix, 0);

        // Adding another screen should now have WS 2 as 1 is taken
        wm.set_num_screens(3);
        wm.detect_screens().unwrap();
        assert_eq!(wm.screens.n_screens(), 3);
        assert_eq!(wm.screens.get(0).unwrap().wix, 1);
        assert_eq!(wm.screens.get(1).unwrap().wix, 0);
        assert_eq!(wm.screens.get(2).unwrap().wix, 2);

        // Focus WS 3 on screen 1, drop down to 1 screen: it should still have WS 3
        wm.focus_workspace(&Selector::Index(3)).unwrap();
        wm.set_num_screens(1);
        wm.detect_screens().unwrap();
        assert_eq!(wm.screens.n_screens(), 1);
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
                        layouts: focus_test_layouts(false),
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

                    if $should_layout {
                        assert!(did_layout);
                    } else {
                        assert!(!did_layout);
                    }
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
                layouts: focus_test_layouts(follow_focus),
                focused_border: Color::try_from("#00ff00").unwrap(),
                unfocused_border: Color::try_from("#ff0000").unwrap(),
                ..Default::default()
            };
            let mut wm = WindowManager::new(conf, conn, vec![], logging_error_handler());
            wm.init().unwrap();
            add_n_clients(&mut wm, n_clients, 0);
            if let Some(id) = current {
                wm.state.clients.set_focused(id, &wm.conn);
            }
            wm.clients.modify(target, |c| c.accepts_focus = accepts_focus);
            wm.conn().clear();

            wm.update_focus(target).unwrap();

            assert_eq!(wm.clients.focused_client_id(), expected_focus);
            assert_eq!(wm.conn().calls(), expected_calls);
        }
    }

    #[test]
    fn cycle_screen_updates_active() {
        let mut wm = test_windowmanager(2, vec![]);

        assert_eq!(wm.active_screen_index(), 0);
        wm.cycle_screen(Forward).unwrap();
        assert_eq!(wm.active_screen_index(), 1);
        wm.cycle_screen(Forward).unwrap();
        assert_eq!(wm.active_screen_index(), 1);
    }

    #[test]
    fn cycle_workspace_updates_focused() {
        let mut wm = test_windowmanager(1, vec![]);

        assert_eq!(wm.focused_workspaces(), vec![0]);
        wm.cycle_workspace(Forward).unwrap();
        assert_eq!(wm.focused_workspaces(), vec![1]);
        wm.cycle_workspace(Backward).unwrap();
        wm.cycle_workspace(Backward).unwrap();
        assert_eq!(wm.focused_workspaces(), vec![8]);
    }

    #[test]
    fn drag_workspace_move_focused_workspaces_between_screens() {
        let mut wm = test_windowmanager(2, vec![]);

        assert_eq!(wm.focused_workspaces(), vec![0, 1]);
        wm.drag_workspace(Forward).unwrap();
        assert_eq!(wm.focused_workspaces(), vec![1, 0]);
    }

    #[test]
    fn cycle_client_updates_focus() {
        let mut wm = test_windowmanager(1, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), HashMap::new())
            .unwrap();
        wm.focus_client(&Selector::WinId(0)).unwrap();

        assert_eq!(wm.focused_client_id(), Some(0));
        wm.cycle_client(Backward).unwrap();
        assert_eq!(wm.focused_client_id(), Some(1));
        wm.cycle_client(Backward).unwrap();
        assert_eq!(wm.focused_client_id(), Some(2));
        wm.cycle_client(Backward).unwrap();
        assert_eq!(wm.focused_client_id(), Some(0));
    }

    #[test]
    fn focus_client() {
        let mut wm = test_windowmanager(1, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), HashMap::new())
            .unwrap();

        let focused = wm.focus_client(&Selector::WinId(0));
        assert_eq!(focused.unwrap(), 0);
        let focused = wm.focus_client(&Selector::WinId(42));
        assert!(focused.is_err());
        let focused = wm.focus_client(&Selector::WinId(1));
        assert_eq!(focused.unwrap(), 1);
        let focused = wm.focus_client(&Selector::WinId(42));
        assert!(focused.is_err());
    }

    #[test]
    fn focus_client_no_clients() {
        let mut wm = test_windowmanager(1, vec![]);
        let focused = wm.focus_client(&Selector::WinId(0));
        assert!(focused.is_err());
    }

    #[test]
    fn rotate_clients() {
        let mut wm = test_windowmanager(1, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(wm.active_workspace().client_ids(), vec![2, 1, 0]);
        wm.rotate_clients(Forward).unwrap();
        assert_eq!(wm.active_workspace().client_ids(), vec![0, 2, 1]);
    }

    #[test]
    fn drag_client() {
        let mut wm = test_windowmanager(1, n_clients(4));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(wm.active_workspace().client_ids(), vec![3, 2, 1, 0]);
        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.active_workspace().client_ids(), vec![2, 3, 1, 0]);
        wm.drag_client(Forward).unwrap();
        assert_eq!(wm.active_workspace().client_ids(), vec![2, 1, 3, 0]);
    }

    #[test]
    fn cycle_layout() {
        let mut wm = test_windowmanager(1, vec![]);

        assert_eq!(wm.current_layout_symbol(), "first");
        wm.cycle_layout(Forward).unwrap();
        assert_eq!(wm.current_layout_symbol(), "second");
        wm.cycle_layout(Forward).unwrap();
        assert_eq!(wm.current_layout_symbol(), "first");
    }

    #[test]
    fn focus_workspace() {
        let mut wm = test_windowmanager(1, vec![]);

        assert_eq!(wm.active_workspace().name(), "1");
        wm.focus_workspace(&Selector::Index(3)).unwrap();
        assert_eq!(wm.active_workspace().name(), "4");
        wm.focus_workspace(&Selector::Condition(&|ws| ws.name() == "9"))
            .unwrap();
        assert_eq!(wm.active_workspace().name(), "9");
    }

    #[test]
    fn toggle_workspace() {
        let mut wm = test_windowmanager(1, vec![]);

        wm.focus_workspace(&Selector::Index(1)).unwrap();
        assert_eq!(wm.active_workspace().name(), "2");
        wm.focus_workspace(&Selector::Index(0)).unwrap();
        assert_eq!(wm.active_workspace().name(), "1");
        wm.toggle_workspace().unwrap();
        assert_eq!(wm.active_workspace().name(), "2");
    }

    #[test]
    fn client_to_workspace() {
        let mut wm = test_windowmanager(1, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(wm.active_workspace().client_ids(), vec![2, 1, 0]);
        (0..3).for_each(|_| wm.client_to_workspace(&Selector::Index(1)).unwrap());
        wm.focus_workspace(&Selector::Index(1)).unwrap();
        assert_eq!(wm.active_workspace().client_ids(), vec![0, 1, 2]);
    }

    #[test]
    fn client_to_screen() {
        let mut wm = test_windowmanager(2, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(wm.focused_workspaces(), vec![0, 1]);
        assert_eq!(wm.active_screen_index(), 0);
        assert_eq!(wm.active_workspace().client_ids(), vec![2, 1, 0]);
        wm.client_to_screen(&Selector::Index(1)).unwrap();
        assert_eq!(wm.active_workspace().client_ids(), vec![1, 0]);
        wm.cycle_screen(Forward).unwrap();
        assert_eq!(wm.active_screen_index(), 1);
        assert_eq!(wm.active_workspace().client_ids(), vec![2]);
    }

    #[test]
    fn toggle_client_fullscreen() {
        let mut wm = test_windowmanager(1, n_clients(1));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert!(!wm.client(&Selector::Focused).unwrap().is_fullscreen(),);
        wm.toggle_client_fullscreen(&Selector::Focused).unwrap();
        assert!(wm.client(&Selector::Focused).unwrap().is_fullscreen());
        wm.toggle_client_fullscreen(&Selector::Focused).unwrap();
        assert!(!wm.client(&Selector::Focused).unwrap().is_fullscreen(),);
    }

    #[test]
    fn screen() {
        let mut wm = test_windowmanager(2, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(wm.active_workspace().client_ids(), vec![2, 1, 0]);
        assert_eq!(wm.focused_workspaces(), vec![0, 1]);

        assert_eq!(
            wm.screen(&Selector::Index(0)),
            wm.screen(&Selector::WinId(0)),
        );

        wm.client_to_screen(&Selector::Index(1)).unwrap();

        assert_eq!(
            wm.screen(&Selector::WinId(2)),
            wm.screen(&Selector::Index(1)),
        );
    }

    #[test]
    fn client() {
        let mut wm = test_windowmanager(1, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(wm.client(&Selector::Focused).unwrap().id(), 2);
        assert_eq!(wm.client(&Selector::Index(2)).unwrap().id(), 0);
    }

    #[test]
    fn client_mut() {
        let mut wm = test_windowmanager(1, n_clients(3));
        wm.init().unwrap();
        wm.grab_keys_and_run(test_key_bindings(), test_mouse_bindings())
            .unwrap();

        assert_eq!(
            wm.client(&Selector::Focused).map(|c| c.workspace()),
            Some(0)
        );
        wm.client_mut(&Selector::Focused).unwrap().set_workspace(5);
        assert_eq!(
            wm.client(&Selector::Focused).map(|c| c.workspace()),
            Some(5)
        );
    }
}
