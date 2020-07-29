//! Hook for adding additional functionality around standard WindowManager actions
use crate::client::Client;
use crate::data_types::WinId;
use crate::manager::WindowManager;

pub trait Hook {
    /**
     * Called when a new Client has been created and penrose state has been initialised
     * but before the client has been added to the active Workspace and before any Layouts
     * have been applied.
     * Argument is the newly created Client which can be modified if desired and optionally
     * not passed back to penrose. If the hook takes ownership of the client, it is responsible
     * ensuring that it is unmapped.
     */
    fn new_client(&mut self, _wm: &mut WindowManager, c: Client) -> Option<Client> {
        Some(c)
    }

    /**
     * Called when a Client is removed from the WindowManager, either through a user initiated
     * kill_client action or the Client exiting itself.
     */
    fn remove_client(&mut self, _wm: &mut WindowManager, _id: WinId) {}

    /**
     * Called after a Layout is applied to the active Workspace.
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
     * Called after focus moves to a new Screen.
     * Argument is a index into the WindowManager screen array (internal data structure that supports
     * indexing) for the new Screen.
     */
    fn screen_change(&mut self, _wm: &mut WindowManager, _screen_index: usize) {}

    /**
     * Called after a new Client gains focus.
     * Argument is the focused Client ID which can be used to fetch the internal Client state if
     * needed.
     */
    fn focus_change(&mut self, _wm: &mut WindowManager, _id: WinId) {}
}
