//! Base widgets for building more complex structures
use crate::{
    core::{hooks::Hook, xconnection::XConn},
    draw::{Color, DrawContext, DrawError, Result, TextStyle, Widget},
};

/// A simple piece of static text with an optional background color.
///
/// Can be used as a simple static element in a status bar or as an inner element for rendering
/// more complex text based widgets.
#[derive(Clone, Debug)]
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
            require_draw: false,
        }
    }

    /// Borrows the current contents of the widget.
    pub fn get_text(&self) -> &String {
        &self.txt
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

    /// Force this text widget to redraw on the next render request.
    /// Mostly used when being wrapped by another widget.
    pub fn force_draw(&mut self) {
        self.require_draw = true;
    }
}

impl<X> Hook<X> for Text where X: XConn {}

impl Widget for Text {
    fn draw(&mut self, ctx: &mut dyn DrawContext, _: usize, _: bool, w: f64, h: f64) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.color(&color);
            ctx.rectangle(0.0, 0.0, w, h);
        }

        let (ew, eh) = self.current_extent(ctx, h)?;
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

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, _h: f64) -> Result<(f64, f64)> {
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

/// A set of lines that highlights the currently selected line.
#[derive(Clone, Debug)]
pub struct LinesWithSelection {
    lines: Vec<String>,
    /// The current selected index
    pub selected: usize,
    max_lines: usize,
    font: String,
    point_size: i32,
    padding: f64,
    bg: Color,
    fg: Color,
    fg_sel: Color,
    bg_sel: Color,
    require_draw: bool,
    extent: Option<(f64, f64)>,
    greedy: bool,
}

impl LinesWithSelection {
    /// Construct a new [LinesWithSelection]
    pub fn new(
        font: String,
        point_size: i32,
        padding: f64,
        bg: Color,
        fg: Color,
        bg_sel: Color,
        fg_sel: Color,
        greedy: bool,
    ) -> Self {
        Self {
            lines: vec![],
            selected: 0,
            max_lines: 10,
            font,
            point_size,
            padding,
            bg,
            fg,
            bg_sel,
            fg_sel,
            require_draw: false,
            extent: None,
            greedy,
        }
    }

    /// Set the displayed lines and selected index.
    pub fn set_input(&mut self, lines: Vec<String>, selected: usize) -> Result<()> {
        if selected >= lines.len() {
            return Err(DrawError::Raw(format!(
                "index out of bounds: {} >= {}",
                selected,
                lines.len()
            )));
        }

        self.lines = lines;
        self.selected = selected;
        self.require_draw = true;
        self.extent = None;
        Ok(())
    }

    /// Set the maximum number of lines from the input that will be displayed.
    ///
    /// Defaults to 10
    pub fn set_max_lines(&mut self, max_lines: usize) {
        self.max_lines = max_lines;
        self.require_draw = true;
        self.extent = None;
    }

    /// The currently selected line (if there is one)
    pub fn get_selected(&self) -> Option<&str> {
        self.lines.get(self.selected).map(|s| s.as_ref())
    }
}

impl<X> Hook<X> for LinesWithSelection where X: XConn {}

impl Widget for LinesWithSelection {
    fn draw(
        &mut self,
        ctx: &mut dyn DrawContext,
        _screen: usize,
        _screen_has_focus: bool,
        w: f64,
        h: f64,
    ) -> Result<()> {
        ctx.color(&self.bg);
        ctx.rectangle(0.0, 0.0, w, h);
        ctx.font(&self.font, self.point_size)?;
        ctx.translate(self.padding, self.padding);

        for (ix, line) in self.lines.iter().enumerate() {
            let (lw, lh) = ctx.text_extent(line)?;
            let fg = if ix == self.selected {
                ctx.color(&self.bg_sel);
                ctx.rectangle(0.0, 0.0, lw + self.padding * 2.0, lh);
                self.fg_sel
            } else {
                self.fg
            };

            ctx.color(&fg);
            ctx.text(line, 0.0, (self.padding, self.padding))?;
            ctx.translate(0.0, lh);
        }

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, _h: f64) -> Result<(f64, f64)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let mut height = 0.0;
                let mut w_max = 0.0;
                for line in self.lines.iter() {
                    ctx.font(&self.font, self.point_size)?;
                    let (w, h) = ctx.text_extent(line)?;
                    height += h;
                    w_max = if w > w_max { w } else { w_max };
                }

                let ext = (w_max + self.padding * 2.0, height + self.padding * 2.0);
                self.extent = Some(ext);
                Ok(ext)
            }
        }
    }

    fn require_draw(&self) -> bool {
        self.require_draw
    }

    fn is_greedy(&self) -> bool {
        self.greedy
    }
}
