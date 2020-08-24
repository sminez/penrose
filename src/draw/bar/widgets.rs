//! Built in status bar widgets
use crate::{
    client::Client,
    data_types::{Selector, WinId},
    draw::{Color, DrawContext, TextStyle, Widget},
    hooks::Hook,
    Result, WindowManager,
};

const PADDING: f64 = 3.0;

/// A simple piece of static text with an optional background color.
///
/// Can be used as a simple static element in a status bar or as an inner element for rendering
/// more complex text based widgets.
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
    /// Construct a new Text
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

    /// Set the rendered text and trigger a redraw
    pub fn set_text(&mut self, txt: impl Into<String>) {
        let new_text = txt.into();
        if self.txt != new_text {
            self.txt = new_text;
            self.extent = None;
            self.require_draw = true;
        }
    }
}

impl Hook for Text {}

impl Widget for Text {
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.color(&color);
            ctx.rectangle(0.0, 0.0, w, h);
        }

        let (ew, eh) = self.extent.unwrap();
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

#[derive(Debug)]
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
        screen: usize,
        style: &TextStyle,
        highlight: impl Into<Color>,
        empty_fg: impl Into<Color>,
    ) -> Self {
        Self {
            workspaces: meta_from_names(workspace_names),
            font: style.font.clone(),
            point_size: style.point_size,
            screen,
            is_focused: screen == 0,
            focused_ws: 0,
            require_draw: false,
            extent: None,
            fg_1: style.fg,
            fg_2: empty_fg.into(),
            bg_1: highlight.into(),
            bg_2: style.bg.unwrap_or(0x000000.into()),
        }
    }

    fn names(&self) -> Vec<&str> {
        self.workspaces.iter().map(|w| w.name.as_ref()).collect()
    }

    fn update_workspace_occupied(&mut self, wm: &mut WindowManager) {
        for ws in self.workspaces.iter_mut() {
            let now_occupied =
                if let Some(ws) = wm.workspace(&Selector::Condition(&|w| w.name() == ws.name)) {
                    ws.len() > 0
                } else {
                    false
                };

            if ws.occupied != now_occupied {
                self.require_draw = true;
                ws.occupied = now_occupied;
            }
        }
    }
}

impl Hook for Workspaces {
    fn new_client(&mut self, _: &mut WindowManager, c: &mut Client) {
        if let Some(ws) = self.workspaces.get_mut(c.workspace()) {
            self.require_draw = ws.occupied == false;
            ws.occupied = true;
        }
    }

    fn remove_client(&mut self, wm: &mut WindowManager, _: WinId) {
        self.update_workspace_occupied(wm);
    }

    fn workspace_change(&mut self, wm: &mut WindowManager, _: usize, new: usize) {
        if self.focused_ws != new {
            self.focused_ws = new;
            if let Some(ws) = self.workspaces.get_mut(new) {
                let res = wm.workspace(&Selector::Condition(&|w| w.name() == ws.name));
                ws.occupied = if let Some(w) = res {
                    w.len() > 0
                } else {
                    false
                };
            }

            self.require_draw = true;
        }
    }

    fn workspaces_updated(&mut self, wm: &mut WindowManager, names: &Vec<&str>, active: usize) {
        if names != &self.names() {
            self.focused_ws = active;
            self.workspaces = meta_from_names(names);
            self.update_workspace_occupied(wm);
            self.extent = None;
            self.require_draw = true;
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
        let (_, eh) = self.extent.unwrap();

        for (i, ws) in self.workspaces.iter().enumerate() {
            if i == self.focused_ws {
                ctx.color(&self.bg_1);
                ctx.rectangle(0.0, 0.0, ws.extent.0, h);
            }

            let fg = if ws.occupied { self.fg_1 } else { self.fg_2 };
            ctx.color(&fg);
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
                    ctx.font(&self.font, self.point_size)?;
                    let (w, h) = ctx.text_extent(&ws.name)?;
                    total += w + PADDING + PADDING;
                    h_max = if h > h_max { h } else { h_max };
                    ws.extent = (w + PADDING + PADDING, h);
                }

                let ext = (total + PADDING, h_max);
                self.extent = Some(ext);
                Ok(ext)
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

/// A text widget that is set via updating the root window name a la dwm
pub struct RootWindowName {
    txt: Text,
}

impl RootWindowName {
    /// Create a new RootWindowName widget
    pub fn new(style: &TextStyle, is_greedy: bool, right_justified: bool) -> Self {
        Self {
            txt: Text::new("penrose", style, is_greedy, right_justified),
        }
    }
}

impl Hook for RootWindowName {
    fn client_name_updated(&mut self, _: &mut WindowManager, _: WinId, name: &str, is_root: bool) {
        if is_root {
            self.txt.set_text(name);
        }
    }
}

impl Widget for RootWindowName {
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()> {
        self.txt.draw(ctx, w, h)
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

/// A text widget that is set via updating the root window name a la dwm
pub struct ActiveWindowName {
    txt: Text,
    max_chars: usize,
}

impl ActiveWindowName {
    /// Create a new ActiveWindowName widget
    pub fn new(
        style: &TextStyle,
        max_chars: usize,
        is_greedy: bool,
        right_justified: bool,
    ) -> Self {
        Self {
            txt: Text::new("", style, is_greedy, right_justified),
            max_chars,
        }
    }

    fn set_text(&mut self, txt: &str) {
        if txt.chars().count() <= self.max_chars {
            self.txt.set_text(txt);
        } else {
            let s: String = txt.chars().take(self.max_chars - 3).collect();
            self.txt.set_text(format!("{}...", s));
        }
    }
}

impl Hook for ActiveWindowName {
    fn remove_client(&mut self, wm: &mut WindowManager, _: WinId) {
        if wm.client(&Selector::Focused) == None {
            self.txt.set_text("");
        }
    }

    fn focus_change(&mut self, wm: &mut WindowManager, id: WinId) {
        if let Some(client) = wm.client(&Selector::WinId(id)) {
            self.set_text(client.wm_name());
        }
    }

    fn client_name_updated(&mut self, wm: &mut WindowManager, id: WinId, name: &str, root: bool) {
        if !root {
            if Some(id) == wm.client(&Selector::Focused).map(|c| c.id()) {
                self.set_text(name);
            }
        }
    }
}

impl Widget for ActiveWindowName {
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()> {
        self.txt.draw(ctx, w, h)
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

/// A simple widget that displays the active layout symbol
pub struct CurrentLayout {
    txt: Text,
}

impl CurrentLayout {
    /// Create a new CurrentLayout widget
    pub fn new(style: &TextStyle) -> Self {
        Self {
            txt: Text::new("", style, false, false),
        }
    }
}

impl Hook for CurrentLayout {
    fn startup(&mut self, wm: &mut WindowManager) {
        self.txt.set_text(wm.current_layout_symbol());
    }

    fn layout_change(&mut self, wm: &mut WindowManager, _: usize, _: usize) {
        self.txt.set_text(wm.current_layout_symbol());
    }

    fn workspace_change(&mut self, wm: &mut WindowManager, _: usize, _: usize) {
        self.txt.set_text(wm.current_layout_symbol());
    }

    fn screen_change(&mut self, wm: &mut WindowManager, _: usize) {
        self.txt.set_text(wm.current_layout_symbol());
    }
}

impl Widget for CurrentLayout {
    fn draw(&mut self, ctx: &mut dyn DrawContext, w: f64, h: f64) -> Result<()> {
        self.txt.draw(ctx, w, h)
    }

    fn current_extent(&mut self, ctx: &mut dyn DrawContext, h: f64) -> Result<(f64, f64)> {
        self.txt.current_extent(ctx, h)
    }

    fn require_draw(&self) -> bool {
        self.txt.require_draw()
    }

    fn is_greedy(&self) -> bool {
        false
    }
}
