//! A lightweight and configurable status bar for penrose
use crate::{core::Draw, Result};
use penrose::{
    core::{State, WindowManager},
    pure::geometry::Rect,
    x::{event::XEvent, Atom, ClientConfig, Prop, WinType, XConn},
    Color, Xid,
};
use std::fmt;
use tracing::{debug, error, info};

pub mod schedule;
pub mod widgets;

use schedule::{run_update_schedules, UpdateSchedule};
use widgets::Widget;

/// The position of a status bar
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Position {
    /// Top of the screen
    Top,
    /// Bottom of the screen
    Bottom,
}

/// A group of [Widget]s and associated point size to use for rendering a [StatusBar] on a single
/// screen.
pub struct PerScreen<X: XConn> {
    point_size: u8,
    h: u32,
    ws: Vec<Box<dyn Widget<X>>>,
}

impl<X: XConn> fmt::Debug for PerScreen<X> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PerScreen")
            .field("point_size", &self.point_size)
            .field("h", &self.h)
            .finish()
    }
}

impl<X: XConn> PerScreen<X> {
    /// Construct a new per-screen set of widgets with an associated point size for the font.
    pub fn new(point_size: u8, h: u32, ws: Vec<Box<dyn Widget<X>>>) -> Self {
        Self { point_size, ws, h }
    }
}

#[derive(Debug)]
enum Widgets<X: XConn> {
    Shared(PerScreen<X>),
    PerScreen(Vec<PerScreen<X>>),
}

impl<X: XConn> Widgets<X> {
    fn for_screen_mut(&mut self, ix: usize) -> &mut PerScreen<X> {
        match self {
            Self::Shared(ps) => ps,
            Self::PerScreen(pss) => {
                let ix = if ix >= pss.len() { pss.len() - 1 } else { ix };
                &mut pss[ix]
            }
        }
    }

    fn for_each_mut<F>(&mut self, n_screens: usize, mut f: F)
    where
        F: FnMut(&mut Box<dyn Widget<X>>),
    {
        match self {
            Self::Shared(ps) => ps.ws.iter_mut().for_each(f),
            Self::PerScreen(pss) => pss
                .iter_mut()
                .take(n_screens) // avoid checking widgets that are not in use
                .for_each(|ps| ps.ws.iter_mut().for_each(&mut f)),
        }
    }

    fn require_draw(&self, n_screens: usize) -> bool {
        match self {
            Self::Shared(ps) => ps.ws.iter().any(|w| w.require_draw()),
            Self::PerScreen(pss) => pss
                .iter()
                .take(n_screens) // avoid checking widgets that are not in use
                .any(|ps| ps.ws.iter().any(|w| w.require_draw())),
        }
    }

    fn update_schedules(&mut self) -> Vec<UpdateSchedule> {
        match self {
            Self::Shared(ps) => ps
                .ws
                .iter_mut()
                .filter_map(|w| w.update_schedule())
                .collect(),
            Self::PerScreen(pss) => pss
                .iter_mut()
                .flat_map(|ps| ps.ws.iter_mut().filter_map(|w| w.update_schedule()))
                .collect(),
        }
    }
}

/// A simple text based status bar that renders a user defined array of [`Widget`]s.
#[derive(Debug)]
pub struct StatusBar<X: XConn> {
    draw: Draw,
    position: Position,
    widgets: Widgets<X>,
    screens: Vec<(Xid, u32)>,
    active_screen: usize,
    font: String,
}

impl<X: XConn> StatusBar<X> {
    /// Try to initialise a new empty status bar. Can fail if we are unable to create a
    /// new window for each bar.
    pub fn try_new(
        position: Position,
        h: u32,
        bg: impl Into<Color>,
        font: &str,
        point_size: u8,
        widgets: Vec<Box<dyn Widget<X>>>,
    ) -> Result<Self> {
        let bg = bg.into();
        let draw = Draw::new(font, point_size, bg)?;

        Ok(Self {
            draw,
            position,
            widgets: Widgets::Shared(PerScreen::new(point_size, h, widgets)),
            screens: vec![],
            active_screen: 0,
            font: font.to_string(),
        })
    }

    /// Try to create a new status bar using a different arrangement of widgets for each screen.
    ///
    /// If more screens are attached than available widget arrangements, the last widget
    /// arrangement will be used as a fallback.
    pub fn try_new_per_screen(
        position: Position,
        bg: impl Into<Color>,
        font: &str,
        widgets: Vec<PerScreen<X>>,
    ) -> Result<Self> {
        let bg = bg.into();
        let point_size = widgets[0].point_size;
        let mut draw = Draw::new(font, point_size, bg)?;
        for &PerScreen { point_size, .. } in widgets.iter() {
            draw.add_font(font, point_size)?;
        }

        Ok(Self {
            draw,
            position,
            widgets: Widgets::PerScreen(widgets),
            screens: vec![],
            active_screen: 0,
            font: font.to_string(),
        })
    }

    /// Add this [`StatusBar`] into the given [`WindowManager`] along with the required
    /// hooks for driving it from the main WindowManager event loop.
    ///
    /// If any [UpdateSchedule]s are requested by [Widgets] then they will be extracted and run as
    /// part of calling this method.
    pub fn add_to(mut self, mut wm: WindowManager<X>) -> WindowManager<X>
    where
        X: 'static,
    {
        let schedules = self.widgets.update_schedules();
        if !schedules.is_empty() {
            run_update_schedules(schedules);
        }

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
            .enumerate()
            .map(|(i, &Rect { x, y, w, h })| {
                let bar_h = self.widgets.for_screen_mut(i).h;
                let y = match self.position {
                    Position::Top => y,
                    Position::Bottom => h - bar_h,
                };

                debug!("creating new window");
                let id = self.draw.new_window(
                    WinType::InputOutput(Atom::NetWindowTypeDock),
                    Rect::new(x, y, w, bar_h),
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

                Ok((id, w))
            })
            .collect::<Result<Vec<(Xid, u32)>>>()?;

        Ok(())
    }

    /// Re-render all widgets in this status bar for a single screen.
    /// Will panic if `i` is out of bounds
    fn redraw_screen(&mut self, i: usize) -> Result<()> {
        let (id, w_screen) = self.screens[i];
        let screen_has_focus = self.active_screen == i;
        let ps = self.widgets.for_screen_mut(i);

        self.draw.set_font(&self.font, ps.point_size)?;
        let mut ctx = self.draw.context_for(id)?;
        ctx.clear()?;

        let mut extents = Vec::new();
        let mut greedy_indices = Vec::new();

        for (j, w) in ps.ws.iter_mut().enumerate() {
            extents.push(w.current_extent(&mut ctx, ps.h)?);
            if w.is_greedy() {
                greedy_indices.push(j)
            }
        }

        let total = extents.iter().map(|(w, _)| w).sum::<u32>();
        let n_greedy = greedy_indices.len();

        if total < w_screen && n_greedy > 0 {
            let per_greedy = (w_screen - total) / n_greedy as u32;
            for i in greedy_indices.iter() {
                let (w, h) = extents[*i];
                extents[*i] = (w + per_greedy, h);
            }
        }

        let mut x = 0;
        for (wd, (w, _)) in ps.ws.iter_mut().zip(extents) {
            wd.draw(&mut ctx, self.active_screen, screen_has_focus, w, ps.h)?;
            x += w;
            ctx.set_x_offset(x as i32);
        }

        self.draw.flush(id)?;

        Ok(())
    }

    /// Re-render all widgets in this status bar for each screen it is displayed on
    pub fn redraw(&mut self) -> Result<()> {
        for i in 0..self.screens.len() {
            self.redraw_screen(i)?;
        }

        Ok(())
    }

    fn redraw_if_needed(&mut self) -> Result<()> {
        if self.widgets.require_draw(self.screens.len()) {
            self.redraw()?;
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
    let n_screens = bar.screens.len();
    bar.widgets.for_each_mut(n_screens, |w| {
        if let Err(e) = w.on_startup(state, x) {
            error!(%e, "error running widget startup hook");
        };
    });

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
    let n_screens = bar.screens.len();
    bar.widgets.for_each_mut(n_screens, |w| {
        if let Err(e) = w.on_refresh(state, x) {
            error!(%e, "error running widget refresh hook");
        }
    });

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
        let screens: Vec<_> = bar.screens.drain(..).collect();

        for (id, _) in screens {
            info!(%id, "removing previous status bar");
            if let Err(e) = bar.draw.destroy_window_and_surface(id) {
                error!(%e, "error when removing previous status bar state");
            }
        }

        if let Err(e) = bar.init_for_screens() {
            error!(%e, "unabled to initialise for screens");
            return Err(penrose::Error::NoScreens);
        }
    }

    bar.active_screen = state.client_set.current_screen().index();
    let n_screens = bar.screens.len();
    bar.widgets.for_each_mut(n_screens, |w| {
        if let Err(e) = w.on_event(event, state, x) {
            error!(%e, "error running widget event hook");
        };
    });

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

    bar.active_screen = state.client_set.current_screen().index();
    let n_screens = bar.screens.len();
    bar.widgets.for_each_mut(n_screens, |w| {
        if let Err(e) = w.on_new_client(id, state, x) {
            error!(%e, "error running widget manage hook");
        }
    });

    if let Err(e) = bar.redraw_if_needed() {
        error!(%e, "error redrawing status bar");
    }

    Ok(())
}
