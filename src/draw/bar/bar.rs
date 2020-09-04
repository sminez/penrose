//! A simple status bar
use crate::{
    client::Client,
    data_types::{Region, WinId},
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
    position: Position,
    widgets: Vec<Box<dyn Widget>>,
    screens: Vec<(WinId, f64)>, // window and width
    hpx: usize,
    h: f64,
    bg: Color,
    active_screen: usize,
}

impl<Ctx: DrawContext> StatusBar<Ctx> {
    /// Try to initialise a new empty status bar. Can fail if we are unable to create our window
    pub fn try_new(
        drw: Box<dyn Draw<Ctx = Ctx>>,
        position: Position,
        h: usize,
        bg: impl Into<Color>,
        fonts: &[&str],
        widgets: Vec<Box<dyn Widget>>,
    ) -> Result<Self> {
        let mut bar = Self {
            drw,
            position,
            widgets,
            screens: vec![],
            hpx: h,
            h: h as f64,
            bg: bg.into(),
            active_screen: 0,
        };
        bar.init_for_screens()?;
        fonts.iter().for_each(|f| bar.drw.register_font(f));

        Ok(bar)
    }

    fn init_for_screens(&mut self) -> Result<()> {
        self.screens = self
            .drw
            .screen_sizes()?
            .iter()
            .map(|r| {
                let (sx, sy, sw, sh) = r.values();
                let y = match self.position {
                    Position::Top => sy as usize,
                    Position::Bottom => sh as usize - self.hpx,
                };
                let id = self
                    .drw
                    .new_window(&WindowType::Dock, sx as usize, y, sw as usize, self.hpx)
                    .unwrap();

                (id, sw as f64)
            })
            .collect();

        Ok(())
    }

    /// Re-render all widgets in this status bar
    pub fn redraw(&mut self) -> Result<()> {
        for (i, &(id, w)) in self.screens.clone().iter().enumerate() {
            let screen_has_focus = self.active_screen == i;
            let mut ctx = self.drw.context_for(id)?;

            ctx.clear();

            ctx.color(&self.bg);
            ctx.rectangle(0.0, 0.0, w, self.h as f64);

            let extents = self.layout(&mut ctx, w)?;
            let mut x = 0.0;
            for (wd, (w, _)) in self.widgets.iter_mut().zip(extents) {
                wd.draw(&mut ctx, self.active_screen, screen_has_focus, w, self.h)?;
                x += w;
                ctx.flush();
                ctx.set_x_offset(x);
            }

            self.drw.flush(id);
        }

        Ok(())
    }

    fn layout(&mut self, ctx: &mut dyn DrawContext, w: f64) -> Result<Vec<(f64, f64)>> {
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

        if total < w && n_greedy > 0 {
            let per_greedy = (w - total) / n_greedy as f64;
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
        self.widgets.iter_mut().for_each(|w| w.new_client(wm, c));
    }

    fn remove_client(&mut self, wm: &mut WindowManager, id: WinId) {
        self.widgets
            .iter_mut()
            .for_each(|w| w.remove_client(wm, id));
    }

    fn client_name_updated(&mut self, wm: &mut WindowManager, id: WinId, s: &str, is_root: bool) {
        self.widgets
            .iter_mut()
            .for_each(|w| w.client_name_updated(wm, id, s, is_root));
    }

    fn layout_applied(&mut self, wm: &mut WindowManager, ws_ix: usize, s_ix: usize) {
        self.widgets
            .iter_mut()
            .for_each(|w| w.layout_applied(wm, ws_ix, s_ix));
    }

    fn layout_change(&mut self, wm: &mut WindowManager, ws_ix: usize, s_ix: usize) {
        self.widgets
            .iter_mut()
            .for_each(|w| w.layout_change(wm, ws_ix, s_ix));
    }

    fn workspace_change(&mut self, wm: &mut WindowManager, prev: usize, new: usize) {
        self.widgets
            .iter_mut()
            .for_each(|w| w.workspace_change(wm, prev, new));
    }

    fn workspaces_updated(&mut self, wm: &mut WindowManager, names: &Vec<&str>, active: usize) {
        self.widgets
            .iter_mut()
            .for_each(|w| w.workspaces_updated(wm, names, active));
    }

    fn screens_updated(&mut self, wm: &mut WindowManager, dimensions: &Vec<Region>) {
        self.screens
            .iter()
            .for_each(|(id, _)| self.drw.destroy_window(*id));
        if let Err(e) = self.init_for_screens() {
            error!("error removing old status bar windows: {}", e)
        }

        self.widgets
            .iter_mut()
            .for_each(|w| w.screens_updated(wm, dimensions));

        // always need to redraw when screen sizes change
        match self.redraw() {
            Ok(_) => (),
            Err(e) => error!("unable to redraw bar: {}", e),
        }
    }

    fn screen_change(&mut self, wm: &mut WindowManager, ix: usize) {
        self.active_screen = ix;
        self.widgets
            .iter_mut()
            .for_each(|w| w.screen_change(wm, ix));
    }

    fn focus_change(&mut self, wm: &mut WindowManager, id: WinId) {
        self.widgets.iter_mut().for_each(|w| w.focus_change(wm, id));
    }

    fn event_handled(&mut self, wm: &mut WindowManager) {
        self.widgets.iter_mut().for_each(|w| w.event_handled(wm));
        self.redraw_if_needed();
    }

    fn startup(&mut self, wm: &mut WindowManager) {
        self.widgets.iter_mut().for_each(|w| w.startup(wm));
        match self.redraw() {
            Ok(_) => (),
            Err(e) => error!("unable to redraw bar: {}", e),
        }
    }
}
