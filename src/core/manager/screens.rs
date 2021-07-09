//! Management of screens
use crate::{
    core::{
        data_types::{Point, Region},
        hooks::HookName,
        manager::event::EventAction,
        ring::{Direction, Ring, Selector},
        screen::Screen,
        xconnection::XState,
    },
    Result,
};
use tracing::{debug, info, trace};

/// State and management of screens being layed out by Penrose.
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ScreenSet {
    screens: Ring<Screen>,
    bar_height: u32,
    top_bar: bool,
}

impl ScreenSet {
    /// Create a new [ScreenSet] by querying the X Server for currently connected displays.
    pub fn new<S>(state: &S, n_workspaces: usize, bar_height: u32, top_bar: bool) -> Result<Self>
    where
        S: XState,
    {
        let mut s = Self {
            screens: Ring::default(),
            bar_height,
            top_bar,
        };

        s.update_known_screens(state, n_workspaces)?;
        Ok(s)
    }

    /// If the requsted workspace index is out of bounds or not currently visible then return None.
    pub fn indexed_screen_for_workspace(&self, wix: usize) -> Option<(usize, &Screen)> {
        self.screens
            .indexed_element(&Selector::Condition(&|s| s.wix == wix))
    }

    /// The ordered list of currently visible [Workspace] indices (one per screen).
    pub fn visible_workspaces(&self) -> Vec<usize> {
        self.screens.vec_map(|s| s.wix)
    }

    /// Get a reference to the first Screen satisfying 'selector'. Xid selectors will return
    /// the screen containing that Client if the client is known.
    /// NOTE: It is not possible to get a mutable reference to a Screen.
    pub fn screen(&self, selector: &Selector<'_, Screen>) -> Option<&Screen> {
        self.screens.element(selector)
    }

    /// The number of detected screens currently being tracked by the WindowManager.
    pub fn n_screens(&self) -> usize {
        self.screens.len()
    }

    /// The current effective screen size of the target screen. Effective screen size is the
    /// physical screen size minus any space reserved for a status bar.
    pub fn screen_size(&self, index: usize, bar_visible: bool) -> Option<Region> {
        self.screens.get(index).map(|s| s.region(bar_visible))
    }

    /// The index of the currently focused screen
    pub fn active_screen_index(&self) -> usize {
        self.screens.focused_index()
    }

    pub(super) fn update_known_screens<S>(
        &mut self,
        state: &S,
        n_workspaces: usize,
    ) -> Result<Vec<EventAction<'_>>>
    where
        S: XState,
    {
        let mut workspace_ordering = self.visible_workspaces();
        workspace_ordering.append(
            &mut (0..n_workspaces)
                .filter(|w| !workspace_ordering.contains(w))
                .collect(),
        );

        debug!(?workspace_ordering, "current workspace ordering");

        let detected: Vec<Screen> = state
            .current_screens()?
            .into_iter()
            .zip(workspace_ordering)
            .enumerate()
            .map(|(ix, (mut s, wix))| {
                s.update_effective_region(self.bar_height, self.top_bar);
                trace!(screen = ix, workspace = wix, "setting workspace for screen");
                s.wix = wix;

                let r = s.region(false);
                info!(index = ix, w = r.w, h = r.h, "screen detected");
                s
            })
            .collect();

        Ok(if self.screens.as_vec() != detected {
            self.screens = Ring::new(detected);
            vec![
                EventAction::LayoutVisible,
                EventAction::RunHook(HookName::ScreenChange),
            ]
        } else {
            vec![]
        })
    }

    pub(super) fn focus_screen(&mut self, sel: &Selector<'_, Screen>) -> Vec<EventAction<'_>> {
        match self.screens.focus(sel) {
            Some((true, focused)) => vec![
                EventAction::SetActiveWorkspace(focused.wix),
                EventAction::RunHook(HookName::ScreenChange),
            ],
            _ => vec![],
        }
    }

    pub(super) fn set_screen_from_point(&mut self, point: Point) -> Vec<EventAction<'_>> {
        self.focus_screen(&Selector::Condition(&|s: &Screen| s.contains(point)))
    }

    pub(super) fn cycle_screen<S>(
        &mut self,
        direction: Direction,
        state: S,
    ) -> Result<Vec<EventAction<'_>>>
    where
        S: XState,
    {
        if !self.screens.would_wrap(direction) {
            self.screens.cycle_focus(direction);
            let focused = self.screens.focused_unchecked();
            state.warp_cursor(None, focused)?;

            Ok(vec![
                EventAction::SetActiveWorkspace(focused.wix),
                EventAction::RunHook(HookName::ScreenChange),
            ])
        } else {
            Ok(vec![])
        }
    }
}
