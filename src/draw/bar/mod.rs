//! Status bars
pub mod bar;
pub mod widgets;

pub use bar::{Position, StatusBar};
pub use widgets::{ActiveWindowName, CurrentLayout, RootWindowName, Text, Workspaces};

use crate::{
    draw::{Color, Draw, DrawContext, TextStyle},
    hooks::Hook,
    Result,
};

const MAX_ACTIVE_WINDOW_CHARS: usize = 80;

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
    fn draw(
        &mut self,
        ctx: &mut dyn DrawContext,
        screen: usize,
        screen_has_focus: bool,
        w: f64,
        h: f64,
    ) -> Result<()>;

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

/// Create a default dwm style status bar that displays content pulled from the
/// WM_NAME property of the root window.
pub fn dwm_bar<Ctx: DrawContext>(
    drw: Box<dyn Draw<Ctx = Ctx>>,
    height: usize,
    style: &TextStyle,
    highlight: impl Into<Color>,
    empty_ws: impl Into<Color>,
    workspaces: &[&str],
) -> Result<StatusBar<Ctx>> {
    let highlight = highlight.into();

    Ok(StatusBar::try_new(
        drw,
        Position::Top,
        height,
        style.bg.unwrap_or(0x000000.into()),
        &[&style.font],
        vec![
            Box::new(Workspaces::new(workspaces, style, highlight, empty_ws)),
            Box::new(CurrentLayout::new(style)),
            Box::new(ActiveWindowName::new(
                &TextStyle {
                    bg: Some(highlight),
                    padding: (6.0, 4.0),
                    ..style.clone()
                },
                MAX_ACTIVE_WINDOW_CHARS,
                true,
                false,
            )),
            Box::new(RootWindowName::new(
                &TextStyle {
                    padding: (4.0, 2.0),
                    ..style.clone()
                },
                false,
                true,
            )),
        ],
    )?)
}
