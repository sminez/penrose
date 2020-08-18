use crate::{
    draw::{Color, DrawContext, Widget},
    hooks::Hook,
    Result,
};

/// A simple piece of static text
pub struct StaticText {
    txt: String,
    font: String,
    point_size: i32,
    fg: Color,
    bg: Option<Color>,
    padding: (f64, f64, f64, f64),
    is_greedy: bool,
    extent: Option<f64>,
}
impl StaticText {
    /// Construct a new StaticText
    pub fn new<S: Into<String>, C: Into<Color>>(
        txt: S,
        font: S,
        point_size: i32,
        fg: C,
        bg: Option<C>,
        padding: (f64, f64, f64, f64),
        is_greedy: bool,
    ) -> Self {
        Self {
            txt: txt.into(),
            font: font.into(),
            point_size,
            fg: fg.into(),
            bg: bg.map(|b| b.into()),
            padding,
            is_greedy,
            extent: None,
        }
    }
}
impl Hook for StaticText {}
impl Widget for StaticText {
    fn draw(&mut self, ctx: &mut Box<&mut dyn DrawContext>, w: f64, h: f64) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.color(&color);
            ctx.rectangle(0.0, 0.0, w, h);
        }
        ctx.font(&self.font, self.point_size)?;
        ctx.color(&self.fg);
        ctx.text(&self.txt, self.padding)?;

        Ok(())
    }

    fn current_extent(&mut self, ctx: &Box<&mut dyn DrawContext>, _h: f64) -> Result<f64> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let extent = ctx.text_extent(&self.txt, &self.font)?;
                self.extent = Some(extent);
                Ok(extent)
            }
        }
    }

    fn require_draw(&self) -> bool {
        false
    }

    fn is_greedy(&self) -> bool {
        self.is_greedy
    }
}
