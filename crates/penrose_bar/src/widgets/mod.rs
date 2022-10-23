//! Self rendering building blocks for text based UI elements
use crate::{Context, Result, TextStyle};
use penrose::{
    core::State,
    x::{XConn, XEvent},
    Color, Xid,
};

pub mod debug;
mod simple;
mod workspaces;

pub use simple::{ActiveWindowName, CurrentLayout, RootWindowName};
pub use workspaces::Workspaces;

/// A status bar widget that can be rendered using a [DrawContext]
pub trait Widget<X>
where
    X: XConn,
{
    /// Render the current state of the widget to the status bar window.
    fn draw(
        &mut self,
        ctx: &mut Context,
        screen: usize,
        screen_has_focus: bool,
        w: f64,
        h: f64,
    ) -> Result<()>;

    /// Current required width and height for this widget due to its content
    fn current_extent(&mut self, ctx: &mut Context, h: f64) -> Result<(f64, f64)>;

    /// Does this widget currently require re-rendering? (should be reset to false when 'draw' is called)
    fn require_draw(&self) -> bool;

    /// If true, this widget will expand to fill remaining available space after layout has been
    /// computed. If multiple greedy widgets are present in a given StatusBar then the available
    /// space will be split evenly between all widgets.
    fn is_greedy(&self) -> bool;

    #[allow(unused_variables)]
    /// A startup hook to be run in order to initialise this Widget
    fn on_startup(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    /// An event hook to be run in order to update this Widget
    fn on_event(&mut self, event: &XEvent, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    /// A refresh hook to be run in order to update this Widget
    fn on_refresh(&mut self, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    /// A manage hook to be run in order to update this Widget
    fn on_new_client(&mut self, id: Xid, state: &mut State<X>, x: &X) -> Result<()> {
        Ok(())
    }
}

/// A simple piece of static text with an optional background color.
///
/// Can be used as a simple static element in a status bar or as an inner element for rendering
/// more complex text based widgets.
#[derive(Clone, Debug, PartialEq)]
pub struct Text {
    txt: String,
    font: String,
    point_size: i32,
    fg: Color,
    bg: Option<Color>,
    padding: (f64, f64),
    is_greedy: bool,
    right_justified: bool,
    extent: Option<(f64, f64)>,
    require_draw: bool,
}

impl Text {
    /// Construct a new [Text]
    pub fn new(
        txt: impl Into<String>,
        style: &TextStyle,
        is_greedy: bool,
        right_justified: bool,
    ) -> Self {
        Self {
            txt: txt.into(),
            font: style.font.clone(),
            point_size: style.point_size,
            fg: style.fg,
            bg: style.bg,
            padding: style.padding,
            is_greedy,
            right_justified,
            extent: None,
            require_draw: true,
        }
    }

    /// Borrow the current contents of the widget.
    pub fn get_text(&self) -> &String {
        &self.txt
    }

    /// Mutably borrow the current contents of the widget.
    pub fn get_text_mut(&mut self) -> &mut String {
        &mut self.txt
    }

    /// Set the rendered text and trigger a redraw
    pub fn set_text(&mut self, txt: impl Into<String>) {
        let new_text = txt.into();
        if self.txt != new_text {
            self.txt = new_text;
            self.extent = None;
            self.require_draw = true;
        }
    }
}

impl<X: XConn> Widget<X> for Text {
    fn draw(&mut self, ctx: &mut Context, _: usize, _: bool, w: f64, h: f64) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.color(&color);
            ctx.rectangle(0.0, 0.0, w, h)?;
        }

        let (ew, eh) = <Self as Widget<X>>::current_extent(self, ctx, h)?;
        ctx.font(&self.font, self.point_size)?;
        ctx.color(&self.fg);

        let offset = w - ew;
        let right_justify = self.right_justified && self.is_greedy && offset > 0.0;
        if right_justify {
            ctx.translate(offset, 0.0);
            ctx.text(&self.txt, h - eh, self.padding)?;
            ctx.translate(-offset, 0.0);
        } else {
            ctx.text(&self.txt, h - eh, self.padding)?;
        }

        self.require_draw = false;

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut Context, _h: f64) -> Result<(f64, f64)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let (l, r) = self.padding;
                ctx.font(&self.font, self.point_size)?;
                let (w, h) = ctx.text_extent(&self.txt)?;
                let extent = (w + l + r, h);
                self.extent = Some(extent);
                Ok(extent)
            }
        }
    }

    fn require_draw(&self) -> bool {
        self.require_draw
    }

    fn is_greedy(&self) -> bool {
        self.is_greedy
    }
}
