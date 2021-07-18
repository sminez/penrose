//! Simple text based status bars
//!
//! This module provides a framework for writing simple, text based status bars such as those seen
//! in `dwm` and `xmonad`. A [StatusBar] itself acts as a multiplexer for the contained widgets and
//! each of their [Hook] triggers, deferring to each individual [HookableWidget] for how it should
//! be rendered to the screen.
//!
//! A minimal example bar configuration is provided in the form of [dwm_bar] which aims to emulate
//! the behaviour and appearance of the built in status bar from `dwm`.
//!
//! Example
//! ```no_run
//! # use penrose::__test_helpers::*;
//! use penrose::core::hooks::Hooks;
//! use penrose::draw::{Color, dwm_bar, TextStyle};
//! use penrose::xcb::{XcbDraw, new_xcb_backed_window_manager};
//!
//! use std::convert::TryFrom;
//!
//! # fn example() -> penrose::Result<()> {
//! let height = 18;
//! let BLACK = "#282828";
//! let WHITE = "#ebdbb2";
//! let GREY = "#3c3836";
//! let BLUE = "#458588";
//! let style = TextStyle {
//!     font: "mono".to_string(),
//!     point_size: 11,
//!     fg: Color::try_from(WHITE)?,
//!     bg: Some(Color::try_from(BLACK)?),
//!     padding: (2.0, 2.0),
//! };
//!
//! let config = Config::default();
//! let hooks: Hooks<_> = vec![
//!     Box::new(dwm_bar(
//!         XcbDraw::new()?,
//!         height,
//!         &style,
//!         Color::try_from(BLUE)?, // highlight
//!         Color::try_from(GREY)?, // empty_ws
//!         config.workspaces().clone(),
//!     )?)
//! ];
//! let mut wm = new_xcb_backed_window_manager(config, hooks, logging_error_handler())?;
//! # Ok(())
//! # }
//! ```
use crate::{
    core::{
        data_types::{Region, WinType},
        hooks::Hook,
        manager::WindowManager,
        xconnection::{Atom, Prop, XConn, Xid},
    },
    draw::{Color, Draw, DrawContext, HookableWidget, Result, TextStyle},
};

use std::fmt;

use crate::draw::widget::{ActiveWindowName, CurrentLayout, RootWindowName, Workspaces};

const MAX_ACTIVE_WINDOW_CHARS: usize = 80;

/// Create a default dwm style status bar that displays content pulled from the
/// WM_NAME property of the root window.
pub fn dwm_bar<C, D, X>(
    drw: D,
    height: usize,
    style: &TextStyle,
    highlight: impl Into<Color>,
    empty_ws: impl Into<Color>,
    workspaces: Vec<impl Into<String>>,
) -> Result<StatusBar<C, D, X>>
where
    C: DrawContext + 'static,
    D: Draw<Ctx = C>,
    X: XConn,
{
    let highlight = highlight.into();
    let workspaces: Vec<String> = workspaces.into_iter().map(|w| w.into()).collect();

    StatusBar::try_new(
        drw,
        Position::Top,
        height,
        style.bg.unwrap_or_else(|| 0x000000.into()),
        &[&style.font],
        vec![
            Box::new(Workspaces::new(&workspaces, style, highlight, empty_ws)),
            Box::new(CurrentLayout::new(style)),
            Box::new(ActiveWindowName::new(
                &TextStyle {
                    bg: Some(highlight),
                    padding: (6.0, 4.0),
                    ..style.clone()
                },
                MAX_ACTIVE_WINDOW_CHARS,
                true,
                false,
            )),
            Box::new(RootWindowName::new(
                &TextStyle {
                    padding: (4.0, 2.0),
                    ..style.clone()
                },
                false,
                true,
            )),
        ],
    )
}

/// The position of a status bar
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Position {
    /// Top of the screen
    Top,
    /// Bottom of the screen
    Bottom,
}

/// A simple status bar that works via hooks
pub struct StatusBar<C, D, X>
where
    C: DrawContext,
    D: Draw<Ctx = C>,
    X: XConn,
{
    drw: D,
    position: Position,
    /// The widgets contained within this status bar
    pub widgets: Vec<Box<dyn HookableWidget<X>>>,
    screens: Vec<(Xid, f64)>, // window and width
    hpx: usize,
    h: f64,
    bg: Color,
    active_screen: usize,
}

impl<C, D, X> fmt::Debug for StatusBar<C, D, X>
where
    C: DrawContext,
    D: Draw<Ctx = C>,
    X: XConn,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StatusBar")
            .field("drw", &stringify!(self.drw))
            .field("position", &self.position)
            .field("widgets", &stringify!(self.widgets))
            .field("screens", &self.screens)
            .field("hpx", &self.hpx)
            .field("bg", &self.bg)
            .field("active_screen", &self.active_screen)
            .finish()
    }
}

impl<C, D, X> StatusBar<C, D, X>
where
    C: DrawContext,
    D: Draw<Ctx = C>,
    X: XConn,
{
    /// Try to initialise a new empty status bar. Can fail if we are unable to create our window
    pub fn try_new(
        drw: D,
        position: Position,
        h: usize,
        bg: impl Into<Color>,
        fonts: &[&str],
        widgets: Vec<Box<dyn HookableWidget<X>>>,
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
        let screen_sizes = self.drw.screen_sizes()?;
        self.screens = screen_sizes
            .iter()
            .map(|r| {
                let (sx, sy, sw, sh) = r.values();
                let y = match self.position {
                    Position::Top => sy as usize,
                    Position::Bottom => sh as usize - self.hpx,
                };
                let id = self.drw.new_window(
                    WinType::InputOutput(Atom::NetWindowTypeDock),
                    Region::new(sx, y as u32, sw, self.hpx as u32),
                    false,
                )?;

                let p = Prop::UTF8String(vec!["penrose-statusbar".to_string()]);
                for atom in &[Atom::NetWmName, Atom::WmName, Atom::WmClass] {
                    self.drw.change_prop(id, atom.as_ref(), p.clone())?;
                }

                self.drw.flush(id)?;
                Ok((id, sw as f64))
            })
            .collect::<Result<Vec<(u32, f64)>>>()?;

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

            self.drw.flush(id)?;
        }

        Ok(())
    }

    fn layout(&mut self, ctx: &mut C, w: f64) -> Result<Vec<(f64, f64)>> {
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
                self.drw.flush(*id)?;
            }
        }

        Ok(())
    }
}

macro_rules! __impl_status_bar_as_hook {
    {
        $($name:ident => $($a:ident: $t:ty),*;)+
    } => {
        impl<C, D, X> Hook<X> for StatusBar<C, D, X>
        where
            C: DrawContext,
            D: Draw<Ctx = C>,
            X: XConn,
        {
            $(fn $name(&mut self, wm: &mut WindowManager<X>, $($a: $t),*) -> crate::Result<()> {
                self.widgets
                    .iter_mut()
                    .try_for_each(|w| w.$name(wm, $($a),*))
            })+

            fn screen_change(&mut self, wm: &mut WindowManager<X>, ix: usize) -> crate::Result<()> {
                self.active_screen = ix;
                self.widgets
                    .iter_mut()
                    .try_for_each(|w| w.screen_change(wm, ix))
            }


            fn screens_updated(&mut self, wm: &mut WindowManager<X>, dimensions: &[Region]) -> crate::Result<()> {
                for (id, _) in self.screens.iter() {
                    self.drw.destroy_client(*id)?;
                }

                if let Err(e) = self.init_for_screens() {
                    error!("error removing old status bar windows: {}", e)
                }

                self.widgets
                    .iter_mut()
                    .try_for_each(|w| w.screens_updated(wm, dimensions))?;

                Ok(self.redraw()?)
            }

            fn event_handled(&mut self, wm: &mut WindowManager<X>) -> crate::Result<()> {
                self.widgets.iter_mut().try_for_each(|w| w.event_handled(wm))?;
                Ok(self.redraw_if_needed()?)
            }

            fn startup(&mut self, wm: &mut WindowManager<X>) -> crate::Result<()>  {
                self.widgets.iter_mut().try_for_each(|w| w.startup(wm))?;
                Ok(self.redraw()?)
            }
        }
    }
}

__impl_status_bar_as_hook! {
    client_name_updated => id: Xid, name: &str, is_root: bool;
    client_added_to_workspace => id: Xid, wix: usize;
    focus_change => id: Xid;
    layout_applied => workspace_index: usize, screen_index: usize;
    layout_change => workspace_index: usize, screen_index: usize;
    new_client => id: Xid;
    randr_notify => ;
    remove_client => id: Xid;
    workspace_change => prev: usize, new: usize;
    workspaces_updated => names: &[&str], active: usize;
}
