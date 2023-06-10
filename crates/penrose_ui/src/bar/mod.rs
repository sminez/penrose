//! A lightweight and configurable status bar for penrose
use crate::{
    core::{Context, Draw},
    Result,
};
use penrose::{
    core::{State, WindowManager},
    pure::geometry::Rect,
    x::{event::XEvent, Atom, ClientConfig, Prop, WinType, XConn},
    Color, Xid,
};
use std::fmt;
use tracing::{debug, error, info};
use x11rb::protocol::xproto::ConnectionExt as _;

pub mod widgets;

use widgets::Widget;

/// The position of a status bar
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Position {
    /// Top of the screen
    Top,
    /// Bottom of the screen
    Bottom,
}

pub struct StatusBar<X: XConn> {
    draw: Draw,
    position: Position,
    widgets: Vec<Box<dyn Widget<X>>>,
    screens: Vec<(Xid, f64)>,
    hpx: u32,
    h: f64,
    bg: Color,
    active_screen: usize,
}

impl<X: XConn> fmt::Debug for StatusBar<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatusBar")
            .field("position", &self.position)
            .field("widgets", &stringify!(self.widgets))
            .field("screens", &self.screens)
            .field("hpx", &self.hpx)
            .field("bg", &self.bg)
            .field("active_screen", &self.active_screen)
            .finish()
    }
}

impl<X: XConn> StatusBar<X> {
    /// Try to initialise a new empty status bar. Can fail if we are unable to create a
    /// new window for each bar.
    pub fn try_new(
        position: Position,
        h: u32,
        bg: impl Into<Color>,
        fonts: &[&str],
        widgets: Vec<Box<dyn Widget<X>>>,
    ) -> Result<Self> {
        let draw = Draw::new()?;

        let mut bar = Self {
            draw,
            position,
            widgets,
            screens: vec![],
            hpx: h,
            h: h as f64,
            bg: bg.into(),
            active_screen: 0,
        };

        fonts.iter().for_each(|f| bar.draw.register_font(f));

        Ok(bar)
    }

    pub fn add_to(self, mut wm: WindowManager<X>) -> WindowManager<X>
    where
        X: 'static,
    {
        wm.state.add_extension(self);
        wm.state.config.compose_or_set_event_hook(event_hook);
        wm.state.config.compose_or_set_manage_hook(manage_hook);
        wm.state.config.compose_or_set_refresh_hook(refresh_hook);
        wm.state.config.compose_or_set_startup_hook(startup_hook);

        wm
    }

    fn init_for_screens(&mut self) -> Result<()> {
        info!("initialising per screen status bar windows");
        let screen_details = self.draw.conn.screen_details()?;

        self.screens = screen_details
            .iter()
            .map(|&Rect { x, y, w, h }| {
                let y = match self.position {
                    Position::Top => y,
                    Position::Bottom => h - self.hpx,
                };

                debug!("creating new window");
                let id = self.draw.new_window(
                    WinType::InputOutput(Atom::NetWindowTypeDock),
                    Rect::new(x, y, w, self.hpx),
                    false,
                )?;

                let data = &[ClientConfig::StackBottom];
                self.draw.conn.set_client_config(id, data)?;

                debug!(%id, "setting props");
                let p = Prop::UTF8String(vec!["penrose-statusbar".to_string()]);
                for atom in &[Atom::NetWmName, Atom::WmName, Atom::WmClass] {
                    self.draw.conn.set_prop(id, atom.as_ref(), p.clone())?;
                }

                debug!("flushing");
                self.draw.flush(id)?;

                Ok((id, w as f64))
            })
            .collect::<Result<Vec<(Xid, f64)>>>()?;

        Ok(())
    }

    /// Re-render all widgets in this status bar
    pub fn redraw(&mut self) -> Result<()> {
        for (i, &(id, w)) in self.screens.clone().iter().enumerate() {
            let screen_has_focus = self.active_screen == i;
            let mut ctx = self.draw.context_for(id)?;

            ctx.clear()?;

            ctx.color(&self.bg);
            ctx.rectangle(0.0, 0.0, w, self.h)?;

            let extents = self.layout(&mut ctx, w)?;
            let mut x = 0.0;
            for (wd, (w, _)) in self.widgets.iter_mut().zip(extents) {
                wd.draw(&mut ctx, self.active_screen, screen_has_focus, w, self.h)?;
                x += w;
                ctx.flush();
                ctx.set_x_offset(x);
            }

            self.draw.flush(id)?;
        }

        Ok(())
    }

    fn layout(&mut self, ctx: &mut Context, w: f64) -> Result<Vec<(f64, f64)>> {
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

    fn redraw_if_needed(&mut self) -> Result<()> {
        if self.widgets.iter().any(|w| w.require_draw()) {
            self.redraw()?;
            for (id, _) in self.screens.iter() {
                self.draw.flush(*id)?;
            }
        }

        Ok(())
    }
}

/// Run any widget startup actions and then redraw
pub fn startup_hook<X: XConn + 'static>(state: &mut State<X>, x: &X) -> penrose::Result<()> {
    let s = state.extension::<StatusBar<X>>()?;
    let mut bar = s.borrow_mut();

    if let Err(e) = bar.init_for_screens() {
        error!(%e, "unabled to initialise for screens");
        return Err(penrose::Error::NoScreens);
    }

    info!("running startup widget hooks");
    for w in bar.widgets.iter_mut() {
        if let Err(e) = w.on_startup(state, x) {
            error!(%e, "error running widget startup hook");
        };
    }

    if let Err(e) = bar.redraw() {
        error!(%e, "error redrawing status bar");
    }

    Ok(())
}

/// Run any widget refresh actions and then redraw if needed
pub fn refresh_hook<X: XConn + 'static>(state: &mut State<X>, x: &X) -> penrose::Result<()> {
    let s = state.extension::<StatusBar<X>>()?;
    let mut bar = s.borrow_mut();

    bar.active_screen = state.client_set.current_screen().index();

    for w in bar.widgets.iter_mut() {
        if let Err(e) = w.on_refresh(state, x) {
            error!(%e, "error running widget refresh hook");
        }
    }

    if let Err(e) = bar.redraw_if_needed() {
        error!(%e, "error redrawing status bar");
    }

    Ok(())
}

/// Run any widget event actions and then redraw if needed
pub fn event_hook<X: XConn + 'static>(
    event: &XEvent,
    state: &mut State<X>,
    x: &X,
) -> penrose::Result<bool> {
    use XEvent::{ConfigureNotify, RandrNotify};

    let s = state.extension::<StatusBar<X>>()?;
    let mut bar = s.borrow_mut();

    if matches!(event, RandrNotify) || matches!(event, ConfigureNotify(e) if e.is_root) {
        info!("screens have changed: recreating status bars");

        for &(id, _) in bar.screens.iter() {
            info!(%id, "removing previous status bar");
            bar.draw.conn.connection().destroy_window(*id)?;
        }

        if let Err(e) = bar.init_for_screens() {
            error!(%e, "unabled to initialise for screens");
            return Err(penrose::Error::NoScreens);
        }
    }

    bar.active_screen = state.client_set.current_screen().index();

    for w in bar.widgets.iter_mut() {
        if let Err(e) = w.on_event(event, state, x) {
            error!(%e, "error running widget event hook");
        };
    }

    if let Err(e) = bar.redraw_if_needed() {
        error!(%e, "error redrawing status bar");
    }

    Ok(true)
}

/// Run any widget on_new_client actions and then redraw if needed
pub fn manage_hook<X: XConn + 'static>(
    id: Xid,
    state: &mut State<X>,
    x: &X,
) -> penrose::Result<()> {
    let s = state.extension::<StatusBar<X>>()?;
    let mut bar = s.borrow_mut();

    for w in bar.widgets.iter_mut() {
        if let Err(e) = w.on_new_client(id, state, x) {
            error!(%e, "error running widget manage hook");
        }
    }

    if let Err(e) = bar.redraw_if_needed() {
        error!(%e, "error redrawing status bar");
    }

    Ok(())
}
