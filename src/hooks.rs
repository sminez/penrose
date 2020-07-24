//! Hook for adding additional functionality around standard WindowManager actions
use crate::client::Client;
use crate::data_types::WinId;
use crate::manager::WindowManager;

pub type WorkspaceIndex = usize;
pub type ScreenIndex = usize;

/**
 * Called when a new Client has been created and penrose state has been initialised
 * but before the client has been added to the active Workspace and before any Layouts
 * have been applied.
 * Argument is the newly created Client which can be modified if desired.
 */
pub type NewClientHook = fn(&mut WindowManager, &mut Client);

/**
 * Called before a Layout is applied to the active Workspace.
 * Arguments are indices into the WindowManager workspace and screen arrays (internal data
 * structures that support indexing) which can be used to fetch references to the active Workspace
 * and Screen.
 */
pub type LayoutHook = fn(&mut WindowManager, WorkspaceIndex, ScreenIndex);

/**
 * Called after the active Workspace is changed on a Screen.
 * Arguments are indices into the WindowManager workspace array (internal data structure that
 * supports indexing) for the previous and new workspace.
 */
pub type WorkspaceChangeHook = fn(&mut WindowManager, WorkspaceIndex, WorkspaceIndex);

/**
 * Called after focus moves to a new Screen.
 * Argument is a index into the WindowManager screen array (internal data structure that supports
 * indexing) for the new Screen.
 */
pub type ScreenChangeHook = fn(&mut WindowManager, ScreenIndex);

/**
 * Called after a new Client gains focus.
 * Argument is the focused Client ID which can be used to fetch the internal Client state if
 * needed.
 */
pub type FocusHook = fn(&mut WindowManager, WinId);
