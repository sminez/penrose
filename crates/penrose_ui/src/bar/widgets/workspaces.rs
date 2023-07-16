//! Widgets for the penrose status bar
use crate::{
    bar::widgets::Widget,
    core::{Context, TextStyle},
    Result,
};
use penrose::{
    core::{ClientSpace, State},
    pure::geometry::Rect,
    x::XConn,
    Color,
};

const PADDING: u32 = 3;

#[derive(Clone, Debug, PartialEq)]
struct WsMeta {
    tag: String,
    occupied: bool,
    extent: (u32, u32),
}

impl WsMeta {
    fn from_state<X: XConn>(state: &State<X>) -> Vec<Self> {
        state
            .client_set
            .ordered_workspaces()
            .map(WsMeta::from)
            .collect()
    }
}

impl From<&ClientSpace> for WsMeta {
    fn from(w: &ClientSpace) -> Self {
        Self {
            tag: w.tag().to_owned(),
            occupied: !w.is_empty(),
            extent: (0, 0),
        }
    }
}

fn focused_workspaces<X: XConn>(state: &State<X>) -> Vec<String> {
    let mut indexed_screens: Vec<(usize, String)> = state
        .client_set
        .screens()
        .map(|s| (s.index(), s.workspace.tag().to_owned()))
        .collect();

    indexed_screens.sort_by_key(|(ix, _)| *ix);

    indexed_screens.into_iter().map(|(_, tag)| tag).collect()
}

/// A simple workspace indicator for a status bar
#[derive(Clone, Debug, PartialEq)]
pub struct Workspaces {
    workspaces: Vec<WsMeta>,
    focused_ws: Vec<String>, // focused ws per screen
    extent: Option<(u32, u32)>,
    fg_1: Color,
    fg_2: Color,
    bg_1: Color,
    bg_2: Color,
    require_draw: bool,
}

impl Workspaces {
    /// Construct a new WorkspaceWidget
    pub fn new(style: TextStyle, highlight: impl Into<Color>, empty_fg: impl Into<Color>) -> Self {
        Self {
            workspaces: vec![],
            focused_ws: vec![], // set in startup hook
            extent: None,
            fg_1: style.fg,
            fg_2: empty_fg.into(),
            bg_1: highlight.into(),
            bg_2: style.bg.unwrap_or_else(|| 0x000000.into()),
            require_draw: true,
        }
    }

    fn tags(&self) -> Vec<&str> {
        self.workspaces.iter().map(|w| w.tag.as_ref()).collect()
    }

    fn update_from_state<X: XConn>(&mut self, state: &State<X>) {
        let wss = WsMeta::from_state(state);
        let focused_ws = focused_workspaces(state);

        let tags_changed = self.tags_changed(&wss);

        if tags_changed {
            self.extent = None;
            self.require_draw = true;
        }

        if self.occupied_changed(&wss) || self.focused_ws != focused_ws {
            self.require_draw = true;
        }

        self.workspaces = wss;
        self.focused_ws = focused_ws;
    }

    fn tags_changed(&self, workspaces: &[WsMeta]) -> bool {
        let new_tags: Vec<&str> = workspaces.iter().map(|w| w.tag.as_ref()).collect();

        self.tags() == new_tags
    }

    // Called after tags_changed above so we assume that tags are matching
    fn occupied_changed(&self, workspaces: &[WsMeta]) -> bool {
        self.workspaces
            .iter()
            .zip(workspaces)
            .any(|(l, r)| l.occupied != r.occupied)
    }

    fn ws_colors(
        &self,
        tag: &str,
        screen: usize,
        screen_has_focus: bool,
        occupied: bool,
    ) -> (Color, Color) {
        let focused_on_this_screen = match &self.focused_ws.get(screen) {
            &Some(focused_tag) => tag == focused_tag,
            None => false,
        };

        let focused = self.focused_ws.iter().any(|t| t == tag);
        let focused_other = focused && !focused_on_this_screen;

        if focused_on_this_screen && screen_has_focus {
            let fg = if occupied { self.fg_1 } else { self.fg_2 };

            (fg, self.bg_1)
        } else if focused {
            let fg = if focused_other { self.bg_1 } else { self.fg_1 };

            (fg, self.fg_2)
        } else {
            let fg = if occupied { self.fg_1 } else { self.fg_2 };

            (fg, self.bg_2)
        }
    }
}

impl<X: XConn> Widget<X> for Workspaces {
    fn draw(
        &mut self,
        ctx: &mut Context<'_>,
        screen: usize,
        screen_has_focus: bool,
        w: u32,
        h: u32,
    ) -> Result<()> {
        ctx.fill_rect(Rect::new(0, 0, w, h), self.bg_2)?;
        ctx.translate(PADDING as i32, 0);
        let (_, eh) = <Self as Widget<X>>::current_extent(self, ctx, h)?;

        for ws in self.workspaces.iter() {
            let (fg, bg) = self.ws_colors(&ws.tag, screen, screen_has_focus, ws.occupied);
            ctx.fill_rect(Rect::new(0, 0, ws.extent.0, h), bg)?;
            ctx.draw_text(&ws.tag, h - eh, (PADDING, PADDING), fg)?;
            ctx.translate(ws.extent.0 as i32, 0);
        }

        self.require_draw = false;

        Ok(())
    }

    fn current_extent(&mut self, ctx: &mut Context<'_>, _h: u32) -> Result<(u32, u32)> {
        match self.extent {
            Some(extent) => Ok(extent),
            None => {
                let mut total = 0;
                let mut h_max = 0;
                for ws in self.workspaces.iter_mut() {
                    let (w, h) = ctx.text_extent(&ws.tag)?;
                    total += w + 2 * PADDING;
                    h_max = if h > h_max { h } else { h_max };
                    ws.extent = (w + 2 * PADDING, h);
                }

                let ext = (total + PADDING, h_max);
                self.extent = Some(ext);

                Ok(ext)
            }
        }
    }

    fn is_greedy(&self) -> bool {
        false
    }

    fn require_draw(&self) -> bool {
        self.require_draw
    }

    fn on_startup(&mut self, state: &mut State<X>, _: &X) -> Result<()> {
        self.update_from_state(state);

        Ok(())
    }

    fn on_refresh(&mut self, state: &mut State<X>, _: &X) -> Result<()> {
        self.update_from_state(state);

        Ok(())
    }
}
