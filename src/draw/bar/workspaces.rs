use crate::{
    data_types::Selector,
    draw::{Color, DrawContext, Widget},
    hooks::Hook,
    Result, WindowManager,
};

struct WSMeta {
    name: String,
    occupied: bool,
    extent: f64,
}

fn meta_from_names(names: &[&str]) -> Vec<WSMeta> {
    names
        .iter()
        .map(|&s| WSMeta {
            name: format!(" {} ", s),
            occupied: false,
            extent: 0.0,
        })
        .collect()
}

/// A simple workspace indicator for a status bar
pub struct WorkspaceWidget {
    workspaces: Vec<WSMeta>,
    font: String,
    point_size: i32,
    screen: usize,
    is_focused: bool,
    focused_ws: usize,
    require_draw: bool,
    extent: Option<f64>,
    fg_1: Color,
    fg_2: Color,
    bg_1: Color,
    bg_2: Color,
}
impl WorkspaceWidget {
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

impl Hook for WorkspaceWidget {
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

impl Widget for WorkspaceWidget {
    fn draw(&mut self, ctx: &mut Box<&mut dyn DrawContext>, w: f64, h: f64) -> Result<()> {
        ctx.color(&self.bg_2);
        ctx.rectangle(0.0, 0.0, w, h);
        ctx.font(&self.font, self.point_size)?;

        let mut offset = 0.0;
        for (i, ws) in self.workspaces.iter().enumerate() {
            if i == self.focused_ws {
                ctx.color(&self.bg_1);
                ctx.rectangle(offset, 0.0, ws.extent, h);
            }

            let fg = if ws.occupied { self.fg_1 } else { self.fg_2 };
            ctx.color(&fg);
            ctx.text(&ws.name, (1.0, 1.0, 1.0, 1.0))?;
            ctx.translate(ws.extent, 0.0);
            offset += ws.extent;
        }

        self.require_draw = false;
        Ok(())
    }

    fn current_extent(&mut self, ctx: &Box<&mut dyn DrawContext>, _h: f64) -> Result<f64> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let mut total = 0.0;
                for ws in self.workspaces.iter_mut() {
                    let extent = ctx.text_extent(&ws.name, &self.font)?;
                    total += extent;
                    ws.extent = extent;
                }

                Ok(total)
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
