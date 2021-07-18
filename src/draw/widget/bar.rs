//! Widgets intended for use in statusbars
use crate::{
    core::{
        data_types::Region,
        hooks::Hook,
        manager::WindowManager,
        ring::Selector,
        xconnection::{XConn, Xid},
    },
    draw::{widget::Text, Color, DrawContext, Result, TextStyle, Widget},
};

const PADDING: f64 = 3.0;

#[derive(Clone, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
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

impl<X> Hook<X> for Workspaces
where
    X: XConn,
{
    fn new_client(&mut self, wm: &mut WindowManager<X>, id: Xid) -> crate::Result<()> {
        let c = wm.client(&Selector::WinId(id)).unwrap();
        if let Some(ws) = self.workspaces.get_mut(c.workspace()) {
            self.require_draw = !ws.occupied;
            ws.occupied = true;
        }

        Ok(())
    }

    fn remove_client(&mut self, wm: &mut WindowManager<X>, _: Xid) -> crate::Result<()> {
        self.update_workspace_occupied(wm);

        Ok(())
    }

    fn client_added_to_workspace(
        &mut self,
        wm: &mut WindowManager<X>,
        _: Xid,
        _: usize,
    ) -> crate::Result<()> {
        self.update_workspace_occupied(wm);

        Ok(())
    }

    fn workspace_change(
        &mut self,
        wm: &mut WindowManager<X>,
        _: usize,
        new: usize,
    ) -> crate::Result<()> {
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

        Ok(())
    }

    fn workspaces_updated(
        &mut self,
        wm: &mut WindowManager<X>,
        names: &[&str],
        _: usize,
    ) -> crate::Result<()> {
        if names != self.names().as_slice() {
            let names: Vec<String> = names.iter().map(|s| s.to_string()).collect();
            self.focused_ws = wm.focused_workspaces();
            self.workspaces = meta_from_names(&names);
            self.update_workspace_occupied(wm);
            self.extent = None;
            self.require_draw = true;
        }

        Ok(())
    }

    fn screen_change(&mut self, _: &mut WindowManager<X>, _: usize) -> crate::Result<()> {
        self.require_draw = true;

        Ok(())
    }

    fn screens_updated(&mut self, wm: &mut WindowManager<X>, _: &[Region]) -> crate::Result<()> {
        self.focused_ws = wm.focused_workspaces();
        self.update_workspace_occupied(wm);
        self.require_draw = true;

        Ok(())
    }

    fn startup(&mut self, wm: &mut WindowManager<X>) -> crate::Result<()> {
        // NOTE: Following initial workspace placement from WindowManager<X>
        self.update_workspace_occupied(wm);
        self.focused_ws = (0..wm.n_screens()).collect();

        Ok(())
    }
}

impl Widget for Workspaces {
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
#[derive(Clone, Debug, PartialEq)]
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

impl<X> Hook<X> for RootWindowName
where
    X: XConn,
{
    fn client_name_updated(
        &mut self,
        _: &mut WindowManager<X>,
        _: Xid,
        name: &str,
        is_root: bool,
    ) -> crate::Result<()> {
        if is_root {
            self.txt.set_text(name);
        }

        Ok(())
    }
}

impl Widget for RootWindowName {
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
#[derive(Clone, Debug, PartialEq)]
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

impl<X> Hook<X> for ActiveWindowName
where
    X: XConn,
{
    fn remove_client(&mut self, wm: &mut WindowManager<X>, _: Xid) -> crate::Result<()> {
        if wm.client(&Selector::Focused) == None {
            self.txt.set_text("");
        }

        Ok(())
    }

    fn focus_change(&mut self, wm: &mut WindowManager<X>, id: Xid) -> crate::Result<()> {
        if let Some(client) = wm.client(&Selector::WinId(id)) {
            self.set_text(client.wm_name());
        }

        Ok(())
    }

    fn client_name_updated(
        &mut self,
        wm: &mut WindowManager<X>,
        id: Xid,
        name: &str,
        root: bool,
    ) -> crate::Result<()> {
        if !root && Some(id) == wm.client(&Selector::Focused).map(|c| c.id()) {
            self.set_text(name);
        }

        Ok(())
    }

    fn screen_change(&mut self, _: &mut WindowManager<X>, _: usize) -> crate::Result<()> {
        self.txt.force_draw();
        Ok(())
    }
}

impl Widget for ActiveWindowName {
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
#[derive(Clone, Debug, PartialEq)]
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

impl<X> Hook<X> for CurrentLayout
where
    X: XConn,
{
    fn startup(&mut self, wm: &mut WindowManager<X>) -> crate::Result<()> {
        self.txt.set_text(wm.current_layout_symbol());
        Ok(())
    }

    fn layout_change(
        &mut self,
        wm: &mut WindowManager<X>,
        _: usize,
        _: usize,
    ) -> crate::Result<()> {
        self.txt.set_text(wm.current_layout_symbol());
        Ok(())
    }

    fn workspace_change(
        &mut self,
        wm: &mut WindowManager<X>,
        _: usize,
        _: usize,
    ) -> crate::Result<()> {
        self.txt.set_text(wm.current_layout_symbol());
        Ok(())
    }

    fn screen_change(&mut self, wm: &mut WindowManager<X>, _: usize) -> crate::Result<()> {
        self.txt.set_text(wm.current_layout_symbol());
        Ok(())
    }
}

impl Widget for CurrentLayout {
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
