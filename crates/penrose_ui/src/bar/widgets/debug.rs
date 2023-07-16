//! Widgets for showing debug information about the current state of penrose
use crate::bar::widgets::{Context, Result, Text, TextStyle, Widget};
use penrose::{
    core::State,
    extensions::util::debug::{summarise_state, CurrentStateConfig},
    x::XConn,
};

/// A text widget that shows the Xid of the current client
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveWindowId {
    inner: Text,
}

impl ActiveWindowId {
    /// Create a new ActiveWindowId widget.
    pub fn new(style: TextStyle, is_greedy: bool, right_justified: bool) -> Self {
        Self {
            inner: Text::new("", style, is_greedy, right_justified),
        }
    }
}

impl<X: XConn> Widget<X> for ActiveWindowId {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
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

/// A text widget that shows a summary of the current Window Manager state.
///
/// Updates on refresh
#[derive(Clone, Debug, PartialEq)]
pub struct StateSummary {
    inner: Text,
    cfg: CurrentStateConfig,
}

impl StateSummary {
    /// Create a new StateSummary widget.
    pub fn new(style: TextStyle) -> Self {
        Self {
            inner: Text::new("", style, false, false),
            cfg: CurrentStateConfig {
                line_per_stat: false,
                ..CurrentStateConfig::default()
            },
        }
    }
}

impl<X: XConn> Widget<X> for StateSummary {
    fn draw(&mut self, ctx: &mut Context<'_>, s: usize, f: bool, w: u32, h: u32) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, h: u32) -> Result<(u32, u32)> {
        Widget::<X>::current_extent(&mut self.inner, ctx, h)
    }

    fn is_greedy(&self) -> bool {
        Widget::<X>::is_greedy(&self.inner)
    }

    fn require_draw(&self) -> bool {
        Widget::<X>::require_draw(&self.inner)
    }

    fn on_refresh(&mut self, state: &mut State<X>, _: &X) -> Result<()> {
        self.inner.set_text(summarise_state(state, &self.cfg));

        Ok(())
    }
}
