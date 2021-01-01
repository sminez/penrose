//! Built in status bar widgets
use crate::{
    core::{
        client::Client,
        data_types::{Region, WinId},
        hooks::Hook,
        manager::WindowManager,
        ring::Selector,
        xconnection::XConn,
    },
    draw::{Color, DrawContext, Result, TextStyle, Widget},
};

use std::marker::PhantomData;

const PADDING: f64 = 3.0;

/// A simple piece of static text with an optional background color.
///
/// Can be used as a simple static element in a status bar or as an inner element for rendering
/// more complex text based widgets.
#[derive(Clone, Debug)]
pub struct Text<X: XConn> {
    phantom: PhantomData<X>,
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

impl<X: XConn> Text<X> {
    /// Construct a new Text
    pub fn new(
        txt: impl Into<String>,
        style: &TextStyle,
        is_greedy: bool,
        right_justified: bool,
    ) -> Self {
        Self {
            phantom: PhantomData,
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

    /// Borrows the current contents of the widget.
    pub fn get_text(&self) -> &String {
        &self.txt
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

impl<X: XConn> Hook<X> for Text<X> {}

impl<X: XConn> Widget<X> for Text<X> {
    fn draw(&mut self, ctx: &mut dyn DrawContext, _: usize, _: bool, w: f64, h: f64) -> Result<()> {
        if let Some(color) = self.bg {
            ctx.color(&color);
            ctx.rectangle(0.0, 0.0, w, h);
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

#[derive(Clone, Debug)]
struct WSMeta {
    name: String,
    occupied: bool,
    extent: (f64, f64),
}

fn meta_from_names(names: &[String]) -> Vec<WSMeta> {
    names
        .iter()
        .map(|s| WSMeta {
            name: s.clone(),
            occupied: false,
            extent: (0.0, 0.0),
        })
        .collect()
}

/// A simple workspace indicator for a status bar
#[derive(Clone, Debug)]
pub struct Workspaces {
    workspaces: Vec<WSMeta>,
    font: String,
    point_size: i32,
    focused_ws: Vec<usize>, // focused ws per screen
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
        workspace_names: &[String],
        style: &TextStyle,
        highlight: impl Into<Color>,
        empty_fg: impl Into<Color>,
    ) -> Self {
        Self {
            workspaces: meta_from_names(workspace_names),
            font: style.font.clone(),
            point_size: style.point_size,
            focused_ws: vec![], // set in startup hook
            require_draw: false,
            extent: None,
            fg_1: style.fg,
            fg_2: empty_fg.into(),
            bg_1: highlight.into(),
            bg_2: style.bg.unwrap_or_else(|| 0x000000.into()),
        }
    }

    fn names(&self) -> Vec<&str> {
        self.workspaces.iter().map(|w| w.name.as_ref()).collect()
    }

    fn update_workspace_occupied<X: XConn>(&mut self, wm: &mut WindowManager<X>) {
        for ws in self.workspaces.iter_mut() {
            let now_occupied =
                if let Some(ws) = wm.workspace(&Selector::Condition(&|w| w.name() == ws.name)) {
                    !ws.is_empty()
                } else {
                    false
                };

            if ws.occupied != now_occupied {
                self.require_draw = true;
                ws.occupied = now_occupied;
            }
        }
    }

    fn ws_colors(
        &self,
        ix: usize,
        screen: usize,
        screen_has_focus: bool,
        occupied: bool,
    ) -> (&Color, Option<&Color>) {
        let focused_here = match self.focused_ws.get(screen) {
            Some(&ws) => ix == ws,
            None => false,
        };
        let focused = self.focused_ws.contains(&ix);
        let focused_other = focused && !focused_here;

        if focused_here && screen_has_focus {
            let fg = if occupied { &self.fg_1 } else { &self.fg_2 };
            (fg, Some(&self.bg_1))
        } else if focused {
            let fg = if focused_other {
                &self.bg_1
            } else {
                &self.fg_1
            };
            (fg, Some(&self.fg_2))
        } else {
            let fg = if occupied { &self.fg_1 } else { &self.fg_2 };
            (fg, None)
        }
    }
}

impl<X: XConn> Hook<X> for Workspaces {
    fn new_client(&mut self, _: &mut WindowManager<X>, c: &mut Client) {
        if let Some(ws) = self.workspaces.get_mut(c.workspace()) {
            self.require_draw = !ws.occupied;
            ws.occupied = true;
        }
    }

    fn remove_client(&mut self, wm: &mut WindowManager<X>, _: WinId) {
        self.update_workspace_occupied(wm);
    }

    fn client_added_to_workspace(&mut self, wm: &mut WindowManager<X>, _: WinId, _: usize) {
        self.update_workspace_occupied(wm);
    }

    fn workspace_change(&mut self, wm: &mut WindowManager<X>, _: usize, new: usize) {
        let screen = wm.active_screen_index();
        if self.focused_ws[screen] != new {
            self.focused_ws[screen] = new;
            if let Some(ws) = self.workspaces.get_mut(new) {
                let res = wm.workspace(&Selector::Condition(&|w| w.name() == ws.name));
                ws.occupied = if let Some(w) = res {
                    !w.is_empty()
                } else {
                    false
                };
            }

            self.require_draw = true;
        }
    }

    fn workspaces_updated(&mut self, wm: &mut WindowManager<X>, names: &[&str], _: usize) {
        if names != self.names().as_slice() {
            let names: Vec<String> = names.iter().map(|s| s.to_string()).collect();
            self.focused_ws = wm.focused_workspaces();
            self.workspaces = meta_from_names(&names);
            self.update_workspace_occupied(wm);
            self.extent = None;
            self.require_draw = true;
        }
    }

    fn screen_change(&mut self, _: &mut WindowManager<X>, _: usize) {
        self.require_draw = true;
    }

    fn screens_updated(&mut self, wm: &mut WindowManager<X>, _: &[Region]) {
        self.focused_ws = wm.focused_workspaces();
        self.update_workspace_occupied(wm);
        self.require_draw = true;
    }

    fn startup(&mut self, wm: &mut WindowManager<X>) {
        // NOTE: Following initial workspace placement from WindowManager<X>
        self.focused_ws = (0..wm.n_screens()).collect()
    }
}

impl<X: XConn> Widget<X> for Workspaces {
    fn draw(
        &mut self,
        ctx: &mut dyn DrawContext,
        screen: usize,
        screen_has_focus: bool,
        w: f64,
        h: f64,
    ) -> Result<()> {
        ctx.color(&self.bg_2);
        ctx.rectangle(0.0, 0.0, w, h);
        ctx.font(&self.font, self.point_size)?;
        ctx.translate(PADDING, 0.0);
        let (_, eh) = self.extent.unwrap();

        for (i, ws) in self.workspaces.iter().enumerate() {
            let (fg, bg) = self.ws_colors(i, screen, screen_has_focus, ws.occupied);
            if let Some(c) = bg {
                ctx.color(c);
                ctx.rectangle(0.0, 0.0, ws.extent.0, h);
            }

            ctx.color(fg);
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
#[derive(Clone, Debug)]
pub struct RootWindowName<X: XConn> {
    txt: Text<X>,
}

impl<X: XConn> RootWindowName<X> {
    /// Create a new RootWindowName widget
    pub fn new(style: &TextStyle, is_greedy: bool, right_justified: bool) -> Self {
        Self {
            txt: Text::new("penrose", style, is_greedy, right_justified),
        }
    }
}

impl<X: XConn> Hook<X> for RootWindowName<X> {
    fn client_name_updated(
        &mut self,
        _: &mut WindowManager<X>,
        _: WinId,
        name: &str,
        is_root: bool,
    ) {
        if is_root {
            self.txt.set_text(name);
        }
    }
}

impl<X: XConn> Widget<X> for RootWindowName<X> {
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

/// A text widget that is set via updating the root window name a la dwm
#[derive(Clone, Debug)]
pub struct ActiveWindowName<X: XConn> {
    txt: Text<X>,
    max_chars: usize,
}

impl<X: XConn> ActiveWindowName<X> {
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

impl<X: XConn> Hook<X> for ActiveWindowName<X> {
    fn remove_client(&mut self, wm: &mut WindowManager<X>, _: WinId) {
        if wm.client(&Selector::Focused) == None {
            self.txt.set_text("");
        }
    }

    fn focus_change(&mut self, wm: &mut WindowManager<X>, id: WinId) {
        if let Some(client) = wm.client(&Selector::WinId(id)) {
            self.set_text(client.wm_name());
        }
    }

    fn client_name_updated(
        &mut self,
        wm: &mut WindowManager<X>,
        id: WinId,
        name: &str,
        root: bool,
    ) {
        if !root && Some(id) == wm.client(&Selector::Focused).map(|c| c.id()) {
            self.set_text(name);
        }
    }

    fn screen_change(&mut self, _: &mut WindowManager<X>, _: usize) {
        self.txt.force_draw();
    }
}

impl<X: XConn> Widget<X> for ActiveWindowName<X> {
    fn draw(
        &mut self,
        ctx: &mut dyn DrawContext,
        screen: usize,
        screen_has_focus: bool,
        w: f64,
        h: f64,
    ) -> Result<()> {
        if screen_has_focus {
            self.txt.draw(ctx, screen, screen_has_focus, w, h)
        } else {
            Ok(())
        }
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
#[derive(Clone, Debug)]
pub struct CurrentLayout<X: XConn> {
    txt: Text<X>,
}

impl<X: XConn> CurrentLayout<X> {
    /// Create a new CurrentLayout widget
    pub fn new(style: &TextStyle) -> Self {
        Self {
            txt: Text::new("", style, false, false),
        }
    }
}

impl<X: XConn> Hook<X> for CurrentLayout<X> {
    fn startup(&mut self, wm: &mut WindowManager<X>) {
        self.txt.set_text(wm.current_layout_symbol());
    }

    fn layout_change(&mut self, wm: &mut WindowManager<X>, _: usize, _: usize) {
        self.txt.set_text(wm.current_layout_symbol());
    }

    fn workspace_change(&mut self, wm: &mut WindowManager<X>, _: usize, _: usize) {
        self.txt.set_text(wm.current_layout_symbol());
    }

    fn screen_change(&mut self, wm: &mut WindowManager<X>, _: usize) {
        self.txt.set_text(wm.current_layout_symbol());
    }
}

impl<X: XConn> Widget<X> for CurrentLayout<X> {
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
        false
    }
}
