//! Status bars
pub mod statusbar;
pub mod widgets;

#[doc(inline)]
pub use statusbar::{Position, StatusBar};
#[doc(inline)]
pub use widgets::{ActiveWindowName, CurrentLayout, RootWindowName, Text, Workspaces};

use crate::{
    core::{hooks::Hook, xconnection::XConn},
    draw::{Color, Draw, DrawContext, Result, TextStyle},
};

const MAX_ACTIVE_WINDOW_CHARS: usize = 80;

/**
 * A status bar [Widget] that can be automatically rendered using a [DrawContext] when
 * triggered via [WindowManager][crate::core::manager::WindowManager] [Hook] calls.
 *
 * HookableWidgets should _not_ be manually registered as hooks: they will be automatically
 * registered by the [StatusBar] containing them on startup.
 */
pub trait HookableWidget<X>: Hook<X> + Widget
where
    X: XConn,
{
}

// Blanket implementation for anything that implements both Hook and Widget
impl<X, T> HookableWidget<X> for T
where
    X: XConn,
    T: Hook<X> + Widget,
{
}

/// A status bar widget that can be rendered using a [DrawContext]
pub trait Widget {
    /// Render the current state of the widget to the status bar window.
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
pub fn dwm_bar<C, D, X>(
    drw: D,
    height: usize,
    style: &TextStyle,
    highlight: impl Into<Color>,
    empty_ws: impl Into<Color>,
    workspaces: Vec<impl Into<String>>,
) -> Result<StatusBar<C, D, X>>
where
    C: DrawContext + 'static,
    D: Draw<Ctx = C>,
    X: XConn,
{
    let highlight = highlight.into();
    let workspaces: Vec<String> = workspaces.into_iter().map(|w| w.into()).collect();

    Ok(StatusBar::try_new(
        drw,
        Position::Top,
        height,
        style.bg.unwrap_or_else(|| 0x000000.into()),
        &[&style.font],
        vec![
            Box::new(Workspaces::new(&workspaces, style, highlight, empty_ws)),
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
