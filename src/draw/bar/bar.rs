//! A simple status bar
use crate::{
    client::Client,
    data_types::WinId,
    draw::{Color, Draw, DrawContext, Widget, WindowType},
    hooks::Hook,
    Result, WindowManager,
};

/// The position of a status bar
pub enum Position {
    /// Top of the screen
    Top,
    /// Bottom of the screen
    Bottom,
}

/// A simple status bar that works via hooks
pub struct StatusBar<Ctx> {
    drw: Box<dyn Draw<Ctx = Ctx>>,
    widgets: Vec<Box<dyn Widget>>,
    id: WinId,
    w: f64,
    h: f64,
    bg: Color,
}

impl<Ctx: DrawContext> StatusBar<Ctx> {
    /// Try to initialise a new empty status bar. Can fail if we are unable to create our window
    pub fn try_new(
        mut drw: Box<dyn Draw<Ctx = Ctx>>,
        position: Position,
        screen_index: usize,
        h: usize,
        bg: impl Into<Color>,
        fonts: &[&str],
        widgets: Vec<Box<dyn Widget>>,
    ) -> Result<Self> {
        let (sw, sh) = drw.screen_size(screen_index)?;
        let y = match position {
            Position::Top => 0,
            Position::Bottom => sh - h,
        };
        let id = drw.new_window(&WindowType::Dock, 0, y, sw, h)?;
        let mut bar = Self {
            drw,
            widgets,
            id,
            w: sw as f64,
            h: h as f64,
            bg: bg.into(),
        };

        fonts.iter().for_each(|f| bar.drw.register_font(f));
        bar.redraw()?;

        Ok(bar)
    }

    /// Re-render all widgets in this status bar
    pub fn redraw(&mut self) -> Result<()> {
        let mut ctx = self.drw.context_for(self.id)?;

        ctx.color(&self.bg);
        ctx.rectangle(0.0, 0.0, self.w as f64, self.h as f64);

        let extents = self.layout(&mut ctx)?;
        let mut x = 0.0;
        for (wd, (w, _)) in self.widgets.iter_mut().zip(extents) {
            wd.draw(&mut ctx, w, self.h)?;
            x += w;
            ctx.flush();
            ctx.set_x_offset(x);
        }

        self.drw.flush(self.id);
        Ok(())
    }

    fn layout(&mut self, ctx: &mut dyn DrawContext) -> Result<Vec<(f64, f64)>> {
        let mut extents = Vec::with_capacity(self.widgets.len());
        let mut greedy_indices = vec![];

        for (i, w) in self.widgets.iter_mut().enumerate() {
            extents.push(w.current_extent(ctx, self.h)?);
            if w.is_greedy() {
                greedy_indices.push(i)
            }
        }

        let total = extents.iter().map(|(w, _)| w).sum::<f64>();
        let n_greedy = greedy_indices.len();

        if total < self.w && n_greedy > 0 {
            let per_greedy = (self.w - total) / n_greedy as f64;
            for i in greedy_indices.iter() {
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
    }

    fn remove_client(&mut self, wm: &mut WindowManager, id: WinId) {
        for w in self.widgets.iter_mut() {
            w.remove_client(wm, id);
        }
    }

    fn client_name_updated(
        &mut self,
        wm: &mut WindowManager,
        id: WinId,
        name: &str,
        is_root: bool,
    ) {
        for w in self.widgets.iter_mut() {
            w.client_name_updated(wm, id, name, is_root);
        }
    }

    fn layout_applied(&mut self, wm: &mut WindowManager, ws_ix: usize, s_ix: usize) {
        for w in self.widgets.iter_mut() {
            w.layout_applied(wm, ws_ix, s_ix);
        }
    }

    fn layout_change(&mut self, wm: &mut WindowManager, ws_ix: usize, s_ix: usize) {
        for w in self.widgets.iter_mut() {
            w.layout_change(wm, ws_ix, s_ix);
        }
    }

    fn workspace_change(&mut self, wm: &mut WindowManager, prev: usize, new: usize) {
        for w in self.widgets.iter_mut() {
            w.workspace_change(wm, prev, new);
        }
    }

    fn workspaces_updated(&mut self, wm: &mut WindowManager, names: &Vec<&str>, active: usize) {
        for w in self.widgets.iter_mut() {
            w.workspaces_updated(wm, names, active);
        }
    }

    fn screen_change(&mut self, wm: &mut WindowManager, ix: usize) {
        for w in self.widgets.iter_mut() {
            w.screen_change(wm, ix);
        }
    }

    fn focus_change(&mut self, wm: &mut WindowManager, id: WinId) {
        for w in self.widgets.iter_mut() {
            w.focus_change(wm, id);
        }
    }

    fn event_handled(&mut self, wm: &mut WindowManager) {
        for w in self.widgets.iter_mut() {
            w.event_handled(wm);
        }
        self.redraw_if_needed();
    }

    fn startup(&mut self, wm: &mut WindowManager) {
        for w in self.widgets.iter_mut() {
            w.startup(wm);
        }
        self.redraw_if_needed();
    }
}
