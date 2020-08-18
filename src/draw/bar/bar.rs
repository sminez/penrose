//! A simple status bar
use crate::{
    client::Client,
    data_types::WinId,
    draw::{Color, Draw, DrawContext, Widget, WindowType},
    hooks::Hook,
    Result, WindowManager,
};

/// A simple status bar that works via hooks
pub struct StatusBar<Ctx> {
    drw: Box<dyn Draw<Ctx = Ctx>>,
    widgets: Vec<Box<dyn Widget>>,
    spacing: f64,
    greedy_indices: Vec<usize>,
    id: WinId,
    w: f64,
    h: f64,
    bg: Color,
}
impl<Ctx: DrawContext> StatusBar<Ctx> {
    /// Try to initialise a new empty status bar. Can fail if we are unable to create our window
    pub fn try_new(
        mut drw: Box<dyn Draw<Ctx = Ctx>>,
        spacing: f64,
        top_bar: bool,
        screen_index: usize,
        h: usize,
        bg: impl Into<Color>,
    ) -> Result<Self> {
        let (sw, sh) = drw.screen_size(screen_index)?;
        let y = if top_bar { 0 } else { sh - h };
        let id = drw.new_window(&WindowType::Dock, 0, y, sw, h)?;

        Ok(Self {
            drw,
            spacing,
            widgets: vec![],
            greedy_indices: vec![],
            id,
            w: sw as f64,
            h: h as f64,
            bg: bg.into(),
        })
    }

    /// Register the fonts required by the widgets contained in this bar
    pub fn register_fonts(&mut self, fonts: &[&str]) {
        for font in fonts {
            self.drw.register_font(font);
        }
    }

    /// Add a new widget to the status bar
    pub fn add_widget(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    /// Re-render all widgets in this status bar
    pub fn redraw(&mut self) -> Result<()> {
        let mut ctx = self.drw.context_for(self.id)?;

        ctx.color(&self.bg);
        ctx.rectangle(0.0, 0.0, self.w as f64, self.h as f64);

        let extents = self.layout(&mut ctx)?;
        for (wd, (w, _)) in self.widgets.iter_mut().zip(extents) {
            wd.draw(&mut ctx, w, self.h)?;
            ctx.translate(w + self.spacing, 0.0);
        }

        self.drw.flush();
        Ok(())
    }

    fn layout(&mut self, ctx: &mut dyn DrawContext) -> Result<Vec<(f64, f64)>> {
        let mut extents = Vec::with_capacity(self.widgets.len());
        for w in self.widgets.iter_mut() {
            extents.push(w.current_extent(ctx, self.h)?);
        }

        let total = extents.iter().map(|(w, _)| w).sum::<f64>();
        let n_greedy = self.greedy_indices.len();

        if total < self.w && n_greedy > 0 {
            // Pad out any greedy widgets that we have
            let per_greedy = (self.w - total) / n_greedy as f64;
            for i in self.greedy_indices.iter() {
                let (w, h) = extents[*i];
                extents[*i] = (w + per_greedy, h);
            }
        }

        // Allowing overflow to happen
        Ok(extents)
    }

    fn redraw_if_needed(&mut self) {
        if self.widgets.iter().any(|w| w.require_draw()) {
            match self.redraw() {
                Ok(_) => (),
                Err(e) => error!("unable to redraw bar: {}", e),
            }
        }
    }
}

impl<Ctx: DrawContext> Hook for StatusBar<Ctx> {
    fn new_client(&mut self, wm: &mut WindowManager, c: &mut Client) {
        for w in self.widgets.iter_mut() {
            w.new_client(wm, c);
        }
        self.redraw_if_needed();
    }

    fn remove_client(&mut self, wm: &mut WindowManager, id: WinId) {
        for w in self.widgets.iter_mut() {
            w.remove_client(wm, id);
        }
        self.redraw_if_needed();
    }

    fn layout_change(&mut self, wm: &mut WindowManager, ws_ix: usize, s_ix: usize) {
        for w in self.widgets.iter_mut() {
            w.layout_change(wm, ws_ix, s_ix);
        }
        self.redraw_if_needed();
    }

    fn workspace_change(&mut self, wm: &mut WindowManager, prev: usize, new: usize) {
        for w in self.widgets.iter_mut() {
            w.workspace_change(wm, prev, new);
        }
        self.redraw_if_needed();
    }

    fn screen_change(&mut self, wm: &mut WindowManager, ix: usize) {
        for w in self.widgets.iter_mut() {
            w.screen_change(wm, ix);
        }
        self.redraw_if_needed();
    }

    fn focus_change(&mut self, wm: &mut WindowManager, id: WinId) {
        for w in self.widgets.iter_mut() {
            w.focus_change(wm, id);
        }
        self.redraw_if_needed();
    }
}
