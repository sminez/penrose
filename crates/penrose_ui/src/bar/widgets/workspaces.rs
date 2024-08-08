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

/// The focus state of a given workspace being rendered within a [WorkspacesWidget].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusState {
    /// The workspace is not currently focused on any screen.
    Unfocused,
    /// The workspace is focused on the screen that the widget is rendered on.
    FocusedOnThisScreen,
    /// The workspace is focused on a screen that the widget is not rendered on.
    FocusedOnOtherScreen,
}

impl FocusState {
    /// Whether or not this workspace is focused on any active screen.
    pub fn focused(&self) -> bool {
        matches!(self, Self::FocusedOnOtherScreen | Self::FocusedOnThisScreen)
    }
}

/// A UI implementation for the [WorkspacesWidget] widget.
pub trait WorkspacesUi {
    /// Update the UI properties of the parent [WorkspacesWidget] as part of startup and refresh
    /// hooks.
    ///
    /// The boolean return of this method is used to indicate to the parent widget that a redraw
    /// is now required. If state has not changed since the last time this method was called then
    /// you should return `false` to reduce unnecessary rendering.
    #[allow(unused_variables)]
    fn update_from_state<X>(
        &mut self,
        workspace_meta: &[WsMeta],
        focused_tags: &[String],
        state: &State<X>,
    ) -> bool
    where
        X: XConn,
    {
        false
    }

    /// The current UI tag string to be shown for a given workspace.
    fn ui_tag(&self, workspace_meta: &WsMeta) -> String {
        workspace_meta.tag.clone()
    }

    /// The background color to be used for the parent [WorkspacesWidget].
    fn background_color(&self) -> Color;

    /// The foreground and background color to be used for rendering a given workspace.
    ///
    /// The [FocusState] provided indicates the current state of the workspace itself, while
    /// `screen_has_focus` is used to indicate whether or not the screen the parent
    /// [WorkspacesWidget] is on is currently focused or not.
    fn colors_for_workspace(
        &self,
        workspace_meta: &WsMeta,
        focus_state: FocusState,
        screen_has_focus: bool,
    ) -> (Color, Color);
}

/// The default UI style of a [WorkspacesWidget].
#[derive(Debug, Clone, PartialEq)]
pub struct DefaultUi {
    fg_1: Color,
    fg_2: Color,
    bg_1: Color,
    bg_2: Color,
}

impl DefaultUi {
    fn new(style: TextStyle, highlight: impl Into<Color>, empty_fg: impl Into<Color>) -> Self {
        Self {
            fg_1: style.fg,
            fg_2: empty_fg.into(),
            bg_1: highlight.into(),
            bg_2: style.bg.unwrap_or_else(|| 0x000000.into()),
        }
    }
}

impl WorkspacesUi for DefaultUi {
    fn background_color(&self) -> Color {
        self.bg_2
    }

    fn colors_for_workspace(
        &self,
        &WsMeta { occupied, .. }: &WsMeta,
        focus_state: FocusState,
        screen_has_focus: bool,
    ) -> (Color, Color) {
        use FocusState::*;

        match focus_state {
            FocusedOnThisScreen if screen_has_focus && occupied => (self.fg_1, self.bg_1),
            FocusedOnThisScreen if screen_has_focus => (self.fg_2, self.bg_1),
            FocusedOnThisScreen => (self.fg_1, self.fg_2),
            FocusedOnOtherScreen => (self.bg_1, self.fg_2),
            Unfocused if occupied => (self.fg_1, self.bg_2),
            Unfocused => (self.fg_2, self.bg_2),
        }
    }
}

/// Metadata around the content of a particular workspace within the current
/// window manager state.
#[derive(Clone, Debug, PartialEq)]
pub struct WsMeta {
    tag: String,
    occupied: bool,
    extent: (u32, u32),
}

impl WsMeta {
    /// The tag used by the window manager for this workspace
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// Whether or not this workspace currently contains any clients
    pub fn occupied(&self) -> bool {
        self.occupied
    }

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

/// A simple workspace indicator for a status bar using a default UI and colorscheme
pub type Workspaces = WorkspacesWidget<DefaultUi>;

impl Workspaces {
    /// Construct a new [WorkspacesWidget] using the [DefaultUi].
    pub fn new(style: TextStyle, highlight: impl Into<Color>, empty_fg: impl Into<Color>) -> Self {
        WorkspacesWidget::new_with_ui(DefaultUi::new(style, highlight, empty_fg))
    }
}

/// A simple workspace indicator for a status bar
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspacesWidget<U>
where
    U: WorkspacesUi,
{
    workspaces: Vec<WsMeta>,
    focused_ws: Vec<String>, // focused ws per screen
    extent: Option<(u32, u32)>,
    ui: U,
    require_draw: bool,
}

impl<U> WorkspacesWidget<U>
where
    U: WorkspacesUi,
{
    /// Construct a new [WorkspacesWidget] with the specified [WorkspacesUi] implementation.
    pub fn new_with_ui(ui: U) -> Self {
        Self {
            workspaces: Vec::new(),
            focused_ws: Vec::new(), // set in startup hook
            extent: None,
            ui,
            require_draw: true,
        }
    }

    fn raw_tags(&self) -> Vec<&str> {
        self.workspaces.iter().map(|w| w.tag.as_ref()).collect()
    }

    fn update_from_state<X: XConn>(&mut self, state: &State<X>) {
        let focused_ws = focused_workspaces(state);
        let wss = WsMeta::from_state(state);

        let ui_updated = self.ui.update_from_state(&wss, &focused_ws, state);
        let tags_changed = self.tags_changed(&wss);

        if ui_updated || tags_changed {
            self.require_draw = true;
            self.extent = None;
        } else if self.focused_ws != focused_ws || self.occupied_changed(&wss) {
            self.require_draw = true;
        }

        self.focused_ws = focused_ws;
        self.workspaces = wss;
    }

    fn tags_changed(&self, workspaces: &[WsMeta]) -> bool {
        let new_tags: Vec<&str> = workspaces.iter().map(|w| w.tag.as_ref()).collect();

        self.raw_tags() == new_tags
    }

    // Called after tags_changed above so we assume that tags are matching
    fn occupied_changed(&self, workspaces: &[WsMeta]) -> bool {
        self.workspaces
            .iter()
            .zip(workspaces)
            .any(|(l, r)| l.occupied != r.occupied)
    }

    fn ws_colors(&self, meta: &WsMeta, screen: usize, screen_has_focus: bool) -> (Color, Color) {
        let focused = self.focused_ws.iter().any(|t| t == &meta.tag);
        let focused_on_this_screen = match &self.focused_ws.get(screen) {
            &Some(focused_tag) => &meta.tag == focused_tag,
            None => false,
        };

        let state = match (focused, focused_on_this_screen) {
            (false, _) => FocusState::Unfocused,
            (_, true) => FocusState::FocusedOnThisScreen,
            (true, false) => FocusState::FocusedOnOtherScreen,
        };

        self.ui.colors_for_workspace(meta, state, screen_has_focus)
    }
}

impl<X, U> Widget<X> for WorkspacesWidget<U>
where
    X: XConn,
    U: WorkspacesUi,
{
    fn draw(
        &mut self,
        ctx: &mut Context<'_>,
        screen: usize,
        screen_has_focus: bool,
        w: u32,
        h: u32,
    ) -> Result<()> {
        ctx.fill_rect(Rect::new(0, 0, w, h), self.ui.background_color())?;
        ctx.translate(PADDING as i32, 0);
        let (_, eh) = <Self as Widget<X>>::current_extent(self, ctx, h)?;

        for ws in self.workspaces.iter() {
            let (fg, bg) = self.ws_colors(ws, screen, screen_has_focus);
            ctx.fill_rect(Rect::new(0, 0, ws.extent.0, h), bg)?;
            ctx.draw_text(&self.ui.ui_tag(ws), h - eh, (PADDING, PADDING), fg)?;
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
                    let (w, h) = ctx.text_extent(&self.ui.ui_tag(ws))?;
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
