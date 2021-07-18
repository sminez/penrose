//! Hook for adding additional functionality around standard WindowManager actions
//!
//! # Overview
//!
//! Hooks are the primary way of injecting custom functionality into penrose when you want to go
//! beyond simply binding actions to key presses. There are multiple points in normal
//! [WindowManager] execution that will trigger the running of user defined hooks, during which you
//! will have complete control over the window manager state and (importantly) block the event loop
//! until your hook exits. For details of what hook points are available, see each of the trait
//! methods outlined below. Note that a single [Hook] can register itself to be called at multiple
//! hook points (all, if desired!) and that hooks are allways called in the order that they are
//! registered with the [WindowManager] on init (i.e. the order of the `Vec` itself).
//!
//! # Implementing Hook
//!
//! As an example of how to write a hook and register it, lets implement a simple hook that logs
//! each new client that is added to a particular workspace, noting if we've seen it before or not.
//! Completely pointless, but it will serve as a nice starting point to show what is happening.
//!
//! ```no_run
//! use penrose::{
//!     core::{
//!         hooks::Hook,
//!         xconnection::{XConn, Xid},
//!     },
//!     xcb::XcbConnection,
//!     Config, Result, WindowManager, logging_error_handler
//! };
//!
//! use std::collections::{HashMap, HashSet};
//!
//! use tracing::info;
//!
//! // Start with the struct itself which will contain any internal state we need to track
//! pub struct LogAddedClients {
//!     seen: HashMap<usize, HashSet<Xid>>,
//! }
//!
//! // It is idiomatic for Hooks to provide a `new` method that returns a pre-boxed struct
//! // so that you can add it straight into your hooks Vector in your main.rs
//! impl LogAddedClients {
//!     pub fn new() -> Box<Self> {
//!         Box::new(Self { seen: HashMap::new() })
//!     }
//! }
//!
//! // As we only care about one of the hook points, that is the only method we need to
//! // implement: all other Hook methods for this struct will be no-ops
//! impl<X: XConn> Hook<X> for LogAddedClients {
//!     fn client_added_to_workspace(
//!         &mut self,
//!         wm: &mut WindowManager<X>,
//!         id: Xid,
//!         wix: usize
//!     ) -> Result<()> {
//!         let clients = self.seen.entry(wix).or_insert(HashSet::new());
//!         if clients.contains(&id) {
//!             info!("'{}' has been on '{}' before!", id, wix)
//!         } else {
//!             clients.insert(id);
//!             info!("'{}' was added to '{}' for the first time", id, wix)
//!         };
//!
//!         Ok(())
//!     }
//! }
//!
//! // Now we simply pass our hook to the WindowManager when we create it
//! fn main() -> penrose::Result<()> {
//!     let mut manager = WindowManager::new(
//!         Config::default(),
//!         XcbConnection::new()?,
//!         vec![LogAddedClients::new()],
//!         logging_error_handler()
//!     );
//!
//!     manager.init()?;
//!
//!     // rest of your startup logic here
//!
//!     Ok(())
//! }
//! ```
//!
//! Now, whenever a [Client][4] is added to a [Workspace][1] (either because it has been newly
//! created, or because it has been moved from one workspace to another) our hook will be called,
//! and our log message will be included in the penrose log stream. More complicated hooks can be
//! built that listen to multiple triggers, but most of the time you will likely only need to
//! implement a single method. For an example of a more complex set up, see the [Scratchpad][2]
//! extension which uses multiple hooks to spawn and manage a client program outside of normal
//! `WindowManager` operation.
//!
//! # When hooks are called
//!
//! Each Hook trigger will be called as part of normal execution of `WindowManager` methods at a
//! point that should be relatively intuitive based on the name of the method. Each method provides
//! a more detailed explanation of exactly what conditions it will be called under. If you would
//! like to see exactly which user level actions lead to specific triggers, try turning on `DEBUG`
//! logging in your logging config as part of your **main.rs** and lookk for the "Running <method>
//! hooks" message that each trigger logs out.
//!
//! *Please see the documentation on each of the individual methods for more details.*
//!
//! # WindowManager execution with user defined Hooks
//!
//! As mentioned above, each time a hook trigger point is reached the `WindowManager` stops normal
//! execution (including responding to [XEvents][3]) and each of the registered hooks is called in
//! turn. If the hook implements the method associated with the trigger that has been hit, then
//! your logic will be run and you will have a mutable reference to the current [WindowManager]
//! state, giving you complete control over what happens next. Note that method calls on the
//! `WindowManager` itself will (of course) resolve immediately, but that any actions which
//! generate [XEvents][3] will only be processed once all hooks have run and control has returned to
//! the manager itself.
//!
//! [1]: crate::core::workspace::Workspace
//! [2]: crate::contrib::extensions::scratchpad::Scratchpad
//! [3]: crate::core::xconnection::XEvent
//! [4]: crate::core::client::Client
use crate::{
    core::{
        data_types::Region,
        manager::WindowManager,
        xconnection::{XConn, Xid},
    },
    Result,
};

/// Names of each of the individual hooks that are triggerable in Penrose.
///
/// This enum is used to indicate to the [WindowManager] that a particular hook should now be
/// triggered as the result of some other action that has taken place during execution.
#[non_exhaustive]
#[allow(missing_docs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum HookName {
    Startup,
    NewClient(Xid),
    RemoveClient(Xid),
    ClientAddedToWorkspace(Xid, usize),
    ClientNameUpdated(Xid, String, bool),
    LayoutApplied(usize, usize),
    LayoutChange(usize),
    WorkspaceChange(usize, usize),
    WorkspacesUpdated(Vec<String>, usize),
    ScreenChange,
    ScreenUpdated,
    RanderNotify,
    FocusChange(u32),
    EventHandled,
}

/// Utility type for defining hooks in your penrose configuration.
pub type Hooks<X> = Vec<Box<dyn Hook<X>>>;

/// User defined functionality triggered by [WindowManager] actions.
///
/// impls of [Hook] can be registered to receive events during [WindowManager] operation. Each hook
/// point is documented as individual methods detailing when and how they will be called. All
/// registered hooks will be called for each trigger so the required methods all provide a no-op
/// default implementation that must be overriden to provide functionality. Hooks may subscribe to
/// multiple triggers to implement more complex behaviours and may store additional state.
///
/// *Care should be taken when writing [Hook] impls to ensure that infinite loops are not created by
/// nested triggers and that, where possible, support for other hooks running from the same triggers
/// is possible.*
///
///
/// # Implementing Hook
///
/// For an example of how to write Hooks, please see the [module level][1] documentation.
///
/// Note that you only need to implement the methods for triggers you intended to respond to: all
/// hook methods have a default empty implementation that is ignored by the `WindowManager`.
///
/// [1]: crate::core::hooks
pub trait Hook<X: XConn> {
    /// # Trigger Point
    ///
    /// Called once at [WindowManager] startup in [grab_keys_and_run][1] after setting up signal handlers
    /// and grabbing key / mouse bindings but before entering the main event loop that polls for
    /// [XEvents][2].
    ///
    /// # Example Uses
    ///
    /// When this trigger is reached, the `WindowManager` will have initialised all of its internal
    /// state, including setting up [Workspaces][3] and [Screens][4] so any set up logic for Hooks
    /// that requires access to this should be placed in a `startup` hook as opposed to being
    /// attempted in the `new` method of the hook itself.
    ///
    /// [1]: crate::core::manager::WindowManager::grab_keys_and_run
    /// [2]: crate::core::xconnection::XEvent
    /// [3]: crate::core::workspace::Workspace
    /// [4]: crate::core::screen::Screen
    #[allow(unused_variables)]
    fn startup(&mut self, wm: &mut WindowManager<X>) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called when a new [Client][5] has been created in response to map request and all penrose
    /// specific state has been initialised, but before the client has been added to the active
    /// [Workspace][1] and before any [Layouts][2] have been applied.
    ///
    /// The `client` argument is the newly created Client which can be modified if desired and
    /// optionally marked as [externally_managed][3] which will prevent penrose from adding it to a
    /// workspace. If the hook takes ownership of the client in this way then it is responsible
    /// for ensuring that it mapped and unmapped.
    ///
    /// # Example Uses
    ///
    /// Inspecting newly created clients is the first and most obvious use of this hook but more
    /// advanced actions can be performed if the hook takes ownership of the client. For an
    /// example, see the [Scratchpad][4] extension which uses this hook to capture a spawned client.
    ///
    /// [1]: crate::core::workspace::Workspace
    /// [2]: crate::core::layout::Layout
    /// [3]: crate::core::client::Client::externally_managed
    /// [4]: crate::contrib::extensions::scratchpad::Scratchpad
    /// [5]: crate::core::client::Client
    #[allow(unused_variables)]
    fn new_client(&mut self, wm: &mut WindowManager<X>, id: Xid) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called *after* a [Client][3] is removed from internal [WindowManager] state, either through
    /// a user initiated [kill_client][1] action or the underlying program exiting.
    ///
    /// # Example Uses
    ///
    /// This hook is called after the client has already been removed, so it is not possible to
    /// interact with the client in any way. This is typically used as a companion to
    /// [new_client][2] when managing a target client externally.
    ///
    /// [1]: crate::core::manager::WindowManager::kill_client
    /// [2]: Hook::new_client
    /// [3]: crate::core::client::Client
    #[allow(unused_variables)]
    fn remove_client(&mut self, wm: &mut WindowManager<X>, id: Xid) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called whenever an existing [Client][5] is added to a [Workspace][1]. This includes newly
    /// created clients when they are first mapped and clients being moved between workspaces using
    /// the [client_to_workspace][2] method on [WindowManager].
    ///
    /// # Example Uses
    ///
    /// The built in [status bar][3] widget [Workspaces][4] uses this to keep track of whether or
    /// not each workspace is occupied or not.
    ///
    /// [1]: crate::core::workspace::Workspace
    /// [2]: crate::core::manager::WindowManager::client_to_workspace
    /// [3]: crate::draw::bar::StatusBar
    /// [4]: crate::draw::widget::bar::Workspaces
    /// [5]: crate::core::client::Client
    #[allow(unused_variables)]
    fn client_added_to_workspace(
        &mut self,
        wm: &mut WindowManager<X>,
        id: Xid,
        wix: usize,
    ) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called whenever something updates the WM_NAME or _NET_WM_NAME property on a window.
    /// `is_root == true` indicates that this is the root window that is being modified.
    ///
    /// # Example Uses
    ///
    /// This allows for simple setting / fetching of string data from individual clients or the
    /// root X window. In particular, this allows for a [dwm][1] style API for controlling
    /// something like a status bar by setting the root window name and then reading it inside of a
    /// hook.
    ///
    /// [1]: https://dwm.suckless.org/
    #[allow(unused_variables)]
    fn client_name_updated(
        &mut self,
        wm: &mut WindowManager<X>,
        id: Xid,
        name: &str,
        is_root: bool,
    ) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called after a [Layout][1] is applied to the active Workspace.
    ///
    /// Arguments are indices into the WindowManager workspace and screen rings (internal data
    /// structures that support indexing) which can be used to fetch references to the active [Workspace][2]
    /// and [Screen][3]. Note that this is called for every application of the layout which
    /// includes:
    ///
    ///   - changing the active workspace
    ///   - adding or removing a client from the active workspace
    ///   - updating the main ratio or number of master clients
    ///   - user calls to the [layout_screen][4] method on [WindowManager]
    ///
    /// # Example Uses
    ///
    /// Running logic that applies after windows have been positioned for a given workspace: for
    /// example, ensuring that a particular window is always in a certain position or that it
    /// floats above all other windows.
    ///
    /// [1]: crate::core::layout::Layout
    /// [2]: crate::core::workspace::Workspace
    /// [3]: crate::core::screen::Screen
    /// [4]: crate::core::manager::WindowManager::layout_screen
    #[allow(unused_variables)]
    fn layout_applied(
        &mut self,
        wm: &mut WindowManager<X>,
        workspace_index: usize,
        screen_index: usize,
    ) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called after a workspace's [Layout][1] has been updated via [cycle_layout][4].
    ///
    /// Arguments are indices into the WindowManager workspace and screen rings (internal data
    /// structures that support indexing) which can be used to fetch references to the active [Workspace][2]
    /// and [Screen][3].
    ///
    /// # Example Uses
    ///
    /// Running additional setup logic for more complex layout functions that can not be done when
    /// the layout itself is invoked.
    ///
    /// [1]: crate::core::layout::Layout
    /// [2]: crate::core::workspace::Workspace
    /// [3]: crate::core::screen::Screen
    /// [4]: crate::core::manager::WindowManager::cycle_layout
    #[allow(unused_variables)]
    fn layout_change(
        &mut self,
        wm: &mut WindowManager<X>,
        workspace_index: usize,
        screen_index: usize,
    ) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called after the active [Workspace][1] is changed on a [Screen][2].
    ///
    /// Arguments are indices into the WindowManager workspace ring (internal data structure that
    /// supports indexing) for the previous and new workspace.
    ///
    /// # Example Uses
    ///
    /// Triggering logic when a particular workspace gains or loses focus.
    ///
    /// [1]: crate::core::workspace::Workspace
    /// [2]: crate::core::screen::Screen
    #[allow(unused_variables)]
    fn workspace_change(
        &mut self,
        wm: &mut WindowManager<X>,
        previous_workspace: usize,
        new_workspace: usize,
    ) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called whenever a [Workspace][1] is dynamically added or removed from the list of known
    /// workspaces once penrose is running.
    ///
    /// # Example Uses
    ///
    /// Updating hooks that care about tracking workspaces when the list of available workspaces is
    /// being dynamically updated while the [WindowManager] is running.
    ///
    /// [1]: crate::core::workspace::Workspace
    #[allow(unused_variables)]
    fn workspaces_updated(
        &mut self,
        wm: &mut WindowManager<X>,
        names: &[&str],
        active: usize,
    ) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called after focus moves to a new [Screen][1].
    ///
    /// Argument is a index into the WindowManager screen ring (internal data structure that supports
    /// indexing) for the new Screen.
    ///
    /// # Example Uses
    ///
    /// Tracking which screen is currently focused without needing to poll state in the
    /// `WindowManager`.
    ///
    /// [1]: crate::core::screen::Screen
    #[allow(unused_variables)]
    fn screen_change(&mut self, wm: &mut WindowManager<X>, screen_index: usize) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called when the list of known [Screens][1] is updated via the [detect_screens][2] method on
    /// the `WindowManager`.
    ///
    /// # Example Uses
    ///
    /// Tracking Screen sizes and details without needing to poll / check every time your hook is
    /// called.
    ///
    /// [1]: crate::core::screen::Screen
    /// [2]: crate::core::manager::WindowManager::detect_screens
    #[allow(unused_variables)]
    fn screens_updated(&mut self, wm: &mut WindowManager<X>, dimensions: &[Region]) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called when the underlying [XConn] emitted a [RandrNotify][1] event.
    ///
    /// This hook will run _before_ polling state for newly connected screens and running the
    /// [screens_updated][2] hook.
    ///
    /// # Example Uses
    ///
    /// This is where any logic you want to run when external monitors are added / removed should
    /// be placed.
    ///
    /// [1]: crate::core::xconnection::XEvent::RandrNotify
    /// [2]: Hook::screens_updated
    #[allow(unused_variables)]
    fn randr_notify(&mut self, wm: &mut WindowManager<X>) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called after a [Client][1] gains focus.
    ///
    /// Argument is the focused Client ID which can be used to fetch the internal Client state if
    /// needed.
    ///
    /// # Example Uses
    ///
    /// Updating information about the focused client, such as in the [ActiveWindowName][2] status
    /// bar widget.
    ///
    /// [1]: crate::core::client::Client
    /// [2]: crate::draw::widget::bar::ActiveWindowName
    #[allow(unused_variables)]
    fn focus_change(&mut self, wm: &mut WindowManager<X>, id: Xid) -> Result<()> {
        Ok(())
    }

    /// # Trigger Point
    ///
    /// Called at the bottom of the main [WindowManager] event loop after each [XEvent][1] is handled.
    ///
    /// # Example Uses
    ///
    /// Useful if you want to ensure that all other event processing has taken place before you
    /// take action in response as part of a more complex hook.
    ///
    /// [1]: crate::core::xconnection::XEvent
    #[allow(unused_variables)]
    fn event_handled(&mut self, wm: &mut WindowManager<X>) -> Result<()> {
        Ok(())
    }
}
