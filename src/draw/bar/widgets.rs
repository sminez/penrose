//! Built in status bar widgets
use crate::{
    data_types::Selector,
    draw::{Color, DrawContext, Widget},
    hooks::Hook,
    Result, WindowManager,
};

const PADDING: f64 = 3.0;

/// A simple piece of static text that never updates
pub struct StaticText {
    txt: String,
    font: String,
    point_size: i32,
    fg: Color,
    bg: Option<Color>,
    padding: (f64, f64),
    is_greedy: bool,
    extent: Option<(f64, f64)>,
}

impl StaticText {
    /// Construct a new StaticText
    pub fn new<S: Into<String>, C: Into<Color>>(
        txt: S,
        font: S,
        point_size: i32,
        fg: C,
        bg: Option<C>,
        padding: (f64, f64),
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
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.color(&color);
            let (x, y) = self.padding;
            ctx.rectangle(0.0, 0.0, w + x + y, h);
        }

        let (_, eh) = self.extent.unwrap();
        ctx.font(&self.font, self.point_size)?;
        ctx.color(&self.fg);
        ctx.text(&self.txt, h - eh, self.padding)?;

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, _h: f64) -> Result<(f64, f64)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let (l, r) = self.padding;
                let (w, h) = ctx.text_extent(&self.txt, &self.font)?;
                let extent = (w + l + r, h);
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

struct WSMeta {
    name: String,
    occupied: bool,
    extent: (f64, f64),
}

fn meta_from_names(names: &[&str]) -> Vec<WSMeta> {
    names
        .iter()
        .map(|&s| WSMeta {
            name: s.to_string(),
            occupied: false,
            extent: (0.0, 0.0),
        })
        .collect()
}

/// A simple workspace indicator for a status bar
pub struct Workspaces {
    workspaces: Vec<WSMeta>,
    font: String,
    point_size: i32,
    screen: usize,
    is_focused: bool,
    focused_ws: usize,
    require_draw: bool,
    extent: Option<(f64, f64)>,
    fg_1: Color,
    fg_2: Color,
    bg_1: Color,
    bg_2: Color,
}

impl Workspaces {
    /// Construct a new WorkspaceWidget
    pub fn new(
        workspace_names: &[&str],
        font: impl Into<String>,
        point_size: i32,
        screen: usize,
        occupied_fg: impl Into<Color>,
        empty_fg: impl Into<Color>,
        focused_bg: impl Into<Color>,
        default_bg: impl Into<Color>,
    ) -> Self {
        Self {
            workspaces: meta_from_names(workspace_names),
            font: font.into(),
            point_size,
            screen,
            is_focused: screen == 0,
            focused_ws: 0,
            require_draw: false,
            extent: None,
            fg_1: occupied_fg.into(),
            fg_2: empty_fg.into(),
            bg_1: focused_bg.into(),
            bg_2: default_bg.into(),
        }
    }

    fn names(&self) -> Vec<&str> {
        self.workspaces.iter().map(|w| w.name.as_ref()).collect()
    }
}

impl Hook for Workspaces {
    fn workspace_change(&mut self, _: &mut WindowManager, _prev: usize, new: usize) {
        self.focused_ws = new;
        self.require_draw = true;
    }

    fn workspaces_updated(&mut self, wm: &mut WindowManager, names: &Vec<&str>, active: usize) {
        if names != &self.names() {
            self.focused_ws = active;
            self.workspaces = meta_from_names(names);
            self.workspaces.iter_mut().for_each(|ws| {
                ws.occupied = wm
                    .workspace(&Selector::Condition(&|w| w.name() == ws.name))
                    .unwrap()
                    .len()
                    > 0
            });
        }
    }

    fn screen_change(&mut self, _: &mut WindowManager, ix: usize) {
        let now_focused = ix == self.screen;
        self.require_draw = self.is_focused != now_focused;
        self.is_focused = now_focused;
    }
}

impl Widget for Workspaces {
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()> {
        ctx.color(&self.bg_2);
        ctx.rectangle(0.0, 0.0, w, h);
        ctx.font(&self.font, self.point_size)?;
        ctx.translate(PADDING, 0.0);

        for (i, ws) in self.workspaces.iter().enumerate() {
            if i == self.focused_ws {
                ctx.color(&self.bg_1);
                ctx.rectangle(0.0, 0.0, ws.extent.0, h);
            }

            let fg = if ws.occupied { self.fg_1 } else { self.fg_2 };
            ctx.color(&fg);
            let (_, eh) = self.extent.unwrap();
            ctx.text(&ws.name, h - eh, (PADDING, PADDING))?;
            ctx.translate(ws.extent.0, 0.0);
        }

        self.require_draw = false;
        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, _h: f64) -> Result<(f64, f64)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let mut total = 0.0;
                let mut h_max = 0.0;
                for ws in self.workspaces.iter_mut() {
                    let (w, h) = ctx.text_extent(&ws.name, &self.font)?;
                    total += w + PADDING + PADDING;
                    h_max = if h > h_max { h } else { h_max };
                    ws.extent = (w + PADDING + PADDING, h);
                }

                self.extent = Some((total, h_max));
                Ok((total, h_max))
            }
        }
    }

    fn require_draw(&self) -> bool {
        self.require_draw
    }

    fn is_greedy(&self) -> bool {
        false
    }
}
