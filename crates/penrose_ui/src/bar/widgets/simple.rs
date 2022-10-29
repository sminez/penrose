//! Simple text based widgets built on top of Text
use crate::{
    bar::widgets::{Text, TextStyle, Widget},
    core::Context,
    Result,
};
use penrose::{
    core::State,
    x::{event::PropertyEvent, Atom, XConn, XConnExt, XEvent},
};

/// A text widget that is set via updating the root window name a la dwm
#[derive(Clone, Debug, PartialEq)]
pub struct RootWindowName {
    inner: Text,
}

impl RootWindowName {
    /// Create a new RootWindowName widget
    pub fn new(style: &TextStyle, is_greedy: bool, right_justified: bool) -> Self {
        Self {
            inner: Text::new("penrose", style, is_greedy, right_justified),
        }
    }
}

impl<X: XConn> Widget<X> for RootWindowName {
    fn draw(&mut self, ctx: &mut Context, s: usize, f: bool, w: f64, h: f64) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, f, w, h)
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

    fn on_event(&mut self, event: &XEvent, _: &mut State<X>, x: &X) -> Result<()> {
        let name_props = [Atom::NetWmName.as_ref(), Atom::WmName.as_ref()];

        match event {
            XEvent::PropertyNotify(PropertyEvent {
                id, atom, is_root, ..
            }) if *is_root && name_props.contains(&atom.as_ref()) => {
                self.inner.set_text(x.window_title(*id)?)
            }

            _ => (),
        }

        Ok(())
    }
}

/// A text widget that shows the name of the currently focused window
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveWindowName {
    inner: Text,
    max_chars: usize,
}

impl ActiveWindowName {
    /// Create a new ActiveWindowName widget with a maximum character count.
    ///
    /// max_chars can not be lower than 3.
    pub fn new(
        max_chars: usize,
        style: &TextStyle,
        is_greedy: bool,
        right_justified: bool,
    ) -> Self {
        Self {
            inner: Text::new("", style, is_greedy, right_justified),
            max_chars: max_chars.max(3),
        }
    }

    fn set_text(&mut self, txt: &str) {
        if txt.chars().count() <= self.max_chars {
            self.inner.set_text(txt);
        } else {
            let s: String = txt.chars().take(self.max_chars - 3).collect();
            self.inner.set_text(format!("{}...", s));
        }
    }
}

impl<X: XConn> Widget<X> for ActiveWindowName {
    fn draw(&mut self, ctx: &mut Context, s: usize, focused: bool, w: f64, h: f64) -> Result<()> {
        if focused {
            Widget::<X>::draw(&mut self.inner, ctx, s, focused, w, h)
        } else {
            Ok(())
        }
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

    fn on_refresh(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        if let Some(id) = state.client_set.current_client() {
            self.set_text(&x.window_title(*id)?)
        } else {
            self.set_text("")
        }

        Ok(())
    }

    fn on_event(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<()> {
        let name_props = [Atom::NetWmName.as_ref(), Atom::WmName.as_ref()];

        if let Some(focused) = state.client_set.current_client() {
            match event {
                XEvent::PropertyNotify(PropertyEvent { id, atom, .. })
                    if id == focused && name_props.contains(&atom.as_ref()) =>
                {
                    self.inner.set_text(x.window_title(*id)?)
                }

                _ => (),
            }
        }

        Ok(())
    }
}

/// A text widget that shows the current layout name
#[derive(Clone, Debug, PartialEq)]
pub struct CurrentLayout {
    inner: Text,
}

impl CurrentLayout {
    /// Create a new CurrentLayout widget
    pub fn new(style: &TextStyle) -> Self {
        Self {
            inner: Text::new("", style, false, false),
        }
    }
}

impl<X: XConn> Widget<X> for CurrentLayout {
    fn draw(&mut self, ctx: &mut Context, s: usize, f: bool, w: f64, h: f64) -> Result<()> {
        Widget::<X>::draw(&mut self.inner, ctx, s, f, w, h)
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
        let layout_name = state.client_set.current_workspace().layout_name();
        self.inner.set_text(format!("[{layout_name}]"));

        Ok(())
    }
}
