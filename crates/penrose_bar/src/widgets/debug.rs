//! Widgets for showing debug information about the current state of penrose
use crate::widgets::{Context, Result, Text, TextStyle, Widget};
use penrose::{core::State, x::XConn};

/// A text widget that shows the Xid of the current client
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveWindowId {
    inner: Text,
}

impl ActiveWindowId {
    /// Create a new ActiveWindowId widget.
    pub fn new(style: &TextStyle, is_greedy: bool, right_justified: bool) -> Self {
        Self {
            inner: Text::new("", style, is_greedy, right_justified),
        }
    }
}

impl<X: XConn> Widget<X> for ActiveWindowId {
    fn draw(&mut self, ctx: &mut Context, s: usize, focused: bool, w: f64, h: f64) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, focused, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context, h: f64) -> Result<(f64, f64)> {
        Widget::<X>::current_extent(&mut self.inner, ctx, h)
    }

    fn is_greedy(&self) -> bool {
        Widget::<X>::is_greedy(&self.inner)
    }

    fn require_draw(&self) -> bool {
        Widget::<X>::require_draw(&self.inner)
    }

    fn on_refresh(&mut self, state: &mut State<X>, _: &X) -> Result<()> {
        if let Some(id) = state.client_set.current_client() {
            self.inner.set_text(format!("FOCUS={}", *id))
        } else {
            self.inner.set_text("FOCUS=None")
        }

        Ok(())
    }
}
