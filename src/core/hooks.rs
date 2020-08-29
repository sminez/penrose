//! Hook for adding additional functionality around standard WindowManager actions
use crate::{
    client::Client,
    data_types::{Region, WinId},
    manager::WindowManager,
};

/**
 * impls of Hook can be registered to receive events during WindowManager operation. Each hook
 * point is documented as individual methods detailing when and how they will be called. All Hook
 * impls will be called for each trigger so the required methods all provide a no-op default
 * implementation that must be overriden to provide functionality. Hooks may 'subscribe' to
 * multiple triggers to implement more complex behaviours and may store additional state. Care
 * should be taken when writing Hook impls to ensure that infinite loops are not created by nested
 * triggers and that, where possible, support for other Hooks running from the same triggers is
 * possible.
 */
pub trait Hook {
    /**
     * Called when a new Client has been created and penrose state has been initialised
     * but before the client has been added to the active Workspace and before any Layouts
     * have been applied.
     * Argument is the newly created Client which can be modified if desired and optionally
     * not passed back to penrose. If the hook takes ownership of the client, it is responsible
     * ensuring that it is unmapped.
     */
    fn new_client(&mut self, _wm: &mut WindowManager, _c: &mut Client) {}

    /**
     * Called when a Client is removed from the WindowManager, either through a user initiated
     * kill_client action or the Client exiting itself.
     */
    fn remove_client(&mut self, _wm: &mut WindowManager, _id: WinId) {}

    /**
     * Called whenever something updates the WM_NAME or _NET_WM_NAME property on a window.
     * is_root == true indicates that this is the root window that is being modified
     */
    fn client_name_updated(
        &mut self,
        _wm: &mut WindowManager,
        _id: WinId,
        _name: &str,
        _is_root: bool,
    ) {
    }

    /**
     * Called after a Layout is applied to the active Workspace.
     * Arguments are indices into the WindowManager workspace and screen arrays (internal data
     * structures that support indexing) which can be used to fetch references to the active Workspace
     * and Screen.
     */
    fn layout_applied(
        &mut self,
        _wm: &mut WindowManager,
        _workspace_index: usize,
        _screen_index: usize,
    ) {
    }

    /**
     * Called after a workspace's layout changes
     * Arguments are indices into the WindowManager workspace and screen arrays (internal data
     * structures that support indexing) which can be used to fetch references to the active Workspace
     * and Screen.
     */
    fn layout_change(
        &mut self,
        _wm: &mut WindowManager,
        _workspace_index: usize,
        _screen_index: usize,
    ) {
    }

    /**
     * Called after the active Workspace is changed on a Screen.
     * Arguments are indices into the WindowManager workspace array (internal data structure that
     * supports indexing) for the previous and new workspace.
     */
    fn workspace_change(
        &mut self,
        _wm: &mut WindowManager,
        _previous_workspace: usize,
        _new_workspace: usize,
    ) {
    }

    /**
     * Called when there has been a change to the WindowManager workspace list.
     */
    fn workspaces_updated(&mut self, _wm: &mut WindowManager, _names: &Vec<&str>, _active: usize) {}

    /**
     * Called after focus moves to a new Screen.
     * Argument is a index into the WindowManager screen array (internal data structure that supports
     * indexing) for the new Screen.
     */
    fn screen_change(&mut self, _wm: &mut WindowManager, _screen_index: usize) {}

    /**
     * Called when there has been a change to the WindowManager workspace list.
     */
    fn screens_updated(&mut self, _wm: &mut WindowManager, _dimensions: &Vec<Region>) {}

    /**
     * Called after a new Client gains focus.
     * Argument is the focused Client ID which can be used to fetch the internal Client state if
     * needed.
     */
    fn focus_change(&mut self, _wm: &mut WindowManager, _id: WinId) {}

    /**
     * Called at the end of the main WindowManager event loop once each XEvent has been handled.
     *
     * Usefull if you want to ensure that all other event processing has taken place before you
     * take action in response to another hook.
     */
    fn event_handled(&mut self, _wm: &mut WindowManager) {}

    /**
     * Called once at window manager startup
     */
    fn startup(&mut self, _wm: &mut WindowManager) {}
}
