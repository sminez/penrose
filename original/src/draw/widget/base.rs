//! Base widgets for building more complex structures
use crate::{
    common::bindings::KeyPress,
    core::hooks::Hook,
    draw::{Color, DrawContext, Error, KeyboardControlled, Result, TextStyle, Widget},
    xconnection::XConn,
};

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
            require_draw: false,
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
            ctx.rectangle(0.0, 0.0, w, h)?;
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
#[derive(Clone, Debug, PartialEq)]
pub struct LinesWithSelection {
    lines: Vec<String>,
    selected: usize,
    n_lines: usize,
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
        n_lines: usize,
        greedy: bool,
    ) -> Self {
        Self {
            lines: vec![],
            selected: 0,
            n_lines,
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
    pub fn set_input(&mut self, lines: Vec<String>) -> Result<()> {
        self.lines = lines;
        self.selected = 0;
        self.require_draw = true;
        self.extent = None;
        Ok(())
    }

    /// Set the currently selected index
    ///
    /// # Errors
    /// Fails if the provided index is out of bounds
    pub fn set_selected(&mut self, selected: usize) -> Result<()> {
        if selected >= self.lines.len() {
            return Err(Error::Raw(format!(
                "index out of bounds: {} >= {}",
                selected,
                self.lines.len()
            )));
        }

        self.selected = selected;
        self.require_draw = true;
        Ok(())
    }

    /// Set the maximum number of lines from the input that will be displayed.
    ///
    /// Defaults to 10
    pub fn set_n_lines(&mut self, n_lines: usize) {
        self.n_lines = n_lines;
        self.require_draw = true;
        self.extent = None;
    }

    /// The currently selected line (if there is one)
    pub fn selected(&self) -> Option<&str> {
        self.lines.get(self.selected).map(|s| s.as_ref())
    }

    /// The currently selected index
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// The raw lines held by this widget
    pub fn lines(&self) -> &Vec<String> {
        &self.lines
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
        ctx.rectangle(0.0, 0.0, w, h)?;
        ctx.font(&self.font, self.point_size)?;
        ctx.translate(self.padding, self.padding);

        // Find the block that the current selection is in
        let block = self.selected / self.n_lines;

        self.lines
            .iter()
            .enumerate()
            .skip(block * self.n_lines)
            .take(self.n_lines)
            .try_for_each(|(ix, line)| -> Result<()> {
                let (_, lh) = ctx.text_extent(line)?;
                let fg = if ix == self.selected {
                    ctx.color(&self.bg_sel);
                    ctx.rectangle(0.0, 0.0, w + self.padding * 2.0, lh)?;
                    self.fg_sel
                } else {
                    self.fg
                };

                ctx.color(&fg);
                ctx.text(line, 0.0, (self.padding, self.padding))?;
                ctx.translate(0.0, lh);
                Ok(())
            })?;

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, _h: f64) -> Result<(f64, f64)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let mut height = 0.0;
                let mut w_max = 0.0;
                for (i, line) in self.lines.iter().enumerate() {
                    ctx.font(&self.font, self.point_size)?;
                    let (w, h) = ctx.text_extent(line)?;
                    w_max = if w > w_max { w } else { w_max };

                    if i < self.n_lines {
                        height += h;
                    }
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

impl KeyboardControlled for LinesWithSelection {
    fn handle_keypress(&mut self, k: KeyPress) -> Result<Option<KeyPress>> {
        match k {
            KeyPress::Up => {
                if self.selected > 0 {
                    self.selected -= 1
                }
            }

            KeyPress::Down => {
                if !self.lines.is_empty() && self.selected < self.lines.len() - 1 {
                    self.selected += 1
                }
            }

            _ => return Ok(Some(k)),
        }

        Ok(None)
    }
}

/// A simple text box that can be driven by user keyboard input
#[derive(Clone, Debug, PartialEq)]
pub struct InputBox {
    txt: Text,
}

impl InputBox {
    /// Create a new TextBox widget
    pub fn new(style: &TextStyle, is_greedy: bool, right_justified: bool) -> Self {
        Self {
            txt: Text::new("", style, is_greedy, right_justified),
        }
    }

    /// Borrow the current contents of the widget.
    pub fn get_text(&self) -> &String {
        self.txt.get_text()
    }

    /// Mutably borrow the current contents of the widget.
    pub fn get_text_mut(&mut self) -> &mut String {
        self.txt.get_text_mut()
    }

    /// Set the rendered text and trigger a redraw
    pub fn set_text(&mut self, txt: impl Into<String>) {
        self.txt.set_text(txt);
    }
}

impl<X> Hook<X> for InputBox where X: XConn {}

impl Widget for InputBox {
    fn draw(&mut self, ctx: &mut dyn DrawContext, s: usize, f: bool, w: f64, h: f64) -> Result<()> {
        self.txt.draw(ctx, s, f, w, h)
    }

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, h: f64) -> Result<(f64, f64)> {
        self.txt.current_extent(ctx, h)
    }

    fn require_draw(&self) -> bool {
        self.txt.require_draw()
    }

    fn is_greedy(&self) -> bool {
        self.txt.is_greedy()
    }
}

impl KeyboardControlled for InputBox {
    fn handle_keypress(&mut self, k: KeyPress) -> Result<Option<KeyPress>> {
        match k {
            KeyPress::Backspace => {
                let s = self.get_text_mut();
                if !s.is_empty() {
                    s.pop();
                }
            }
            KeyPress::Utf8(c) => self.get_text_mut().push_str(&c),
            _ => return Ok(Some(k)),
        }

        Ok(None)
    }
}
