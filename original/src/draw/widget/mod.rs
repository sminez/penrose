//! Self rendering building blocks
//!
//! The widgets defined in this module are backed by the [Draw][1] trait and are composable to
//! build up more complex layouts and UI elements.
//!
//! [1]: crate::draw::Draw
use crate::{
    common::bindings::KeyPress,
    core::hooks::Hook,
    draw::{DrawContext, Result},
    xconnection::XConn,
};

pub mod bar;
pub mod base;

#[doc(inline)]
pub use bar::*;
#[doc(inline)]
pub use base::*;

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

/**
 * A status bar [Widget] that can be automatically rendered using a [DrawContext] when
 * triggered via [WindowManager][crate::core::manager::WindowManager] [Hook] calls.
 *
 * HookableWidgets should _not_ be manually registered as hooks: they will be automatically
 * registered by the [StatusBar][crate::draw::StatusBar] containing them on startup.
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

/// Something that can respond to user [KeyPress] events
pub trait KeyboardControlled {
    /// Process the given [KeyPress]
    ///
    /// Should return `Ok(None)` if the [KeyPress] has been handled and no longer
    /// propagated, or `Ok(Some(k))` if further processing is possible.
    fn handle_keypress(&mut self, k: KeyPress) -> Result<Option<KeyPress>>;
}
