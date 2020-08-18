//! Status bars
pub mod bar;
mod text;
mod workspaces;

pub use bar::StatusBar;
pub use text::StaticText;
pub use workspaces::WorkspaceWidget;

use crate::{draw::DrawContext, hooks::Hook, Result};

/**
 * A status bar widget
 *
 * Widgets need to implement Hook but should not be registered with the WindowManager to receive
 * triggers: the status bar itself will handle passing through triggers and check for required
 * updates to the UI.
 */
pub trait Widget: Hook {
    /**
     * Render the current state of the widget to the status bar window.
     */
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()>;

    /// Current required width and height for this widget due to its content
    fn current_extent(&mut self, ctx: &mut dyn DrawContext, h: f64) -> Result<(f64, f64)>;

    /// Does this widget currently require re-rendering? (should be updated when 'draw' is called)
    fn require_draw(&self) -> bool;

    /**
     * If true, this widget will expand to fill remaining available space after layout has been
     * computed. If multiple greedy widgets are present in a given StatusBar then the available
     * space will be split evenly between all widgets.
     */
    fn is_greedy(&self) -> bool;
}
