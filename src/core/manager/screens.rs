//! State and management of screens being layed out by Penrose.
use crate::{
    core::{
        data_types::Region,
        hooks::HookName,
        manager::event::EventAction,
        ring::{Direction, Ring, Selector},
        screen::Screen,
        xconnection::XState,
    },
    Result,
};
use tracing::{debug, info, trace};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub(super) struct Screens {
    pub(super) inner: Ring<Screen>,
    bar_height: u32,
    top_bar: bool,
}

impl Screens {
    pub fn new(bar_height: u32, top_bar: bool) -> Self {
        Self {
            inner: Ring::default(),
            bar_height,
            top_bar,
        }
    }

    pub fn indexed_screen_for_workspace(&self, wix: usize) -> Option<(usize, &Screen)> {
        self.inner
            .indexed_element(&Selector::Condition(&|s| s.wix == wix))
    }

    pub fn focused(&self) -> &Screen {
        // There is always at least one screen attached
        self.inner.focused_unchecked()
    }

    pub fn focused_mut(&mut self) -> &mut Screen {
        // There is always at least one screen attached
        self.inner.focused_mut_unchecked()
    }

    pub fn get(&self, index: usize) -> Option<&Screen> {
        self.inner.get(index)
    }

    pub fn focused_index(&self) -> usize {
        self.inner.focused_index()
    }

    pub fn visible_workspaces(&self) -> Vec<usize> {
        self.inner.vec_map(|s| s.wix)
    }

    pub fn screen(&self, selector: &Selector<'_, Screen>) -> Option<&Screen> {
        self.inner.element(selector)
    }

    pub fn n_screens(&self) -> usize {
        self.inner.len()
    }

    pub fn screen_size(&self, index: usize, bar_visible: bool) -> Option<Region> {
        self.inner.get(index).map(|s| s.region(bar_visible))
    }

    pub fn active_screen_index(&self) -> usize {
        self.inner.focused_index()
    }

    pub fn active_ws_index(&self) -> usize {
        self.inner.focused_unchecked().wix
    }

    pub fn update_known_screens<S>(
        &mut self,
        state: &S,
        n_workspaces: usize,
    ) -> Result<Vec<EventAction>>
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

        let mut detected: Vec<Screen> = state.current_screens()?;

        // sort the screens to be in the correct order
        detected.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let detected = detected
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

        Ok(if self.inner.as_vec() != detected {
            self.inner = Ring::new(detected);
            vec![
                EventAction::LayoutVisible,
                EventAction::RunHook(HookName::ScreenUpdated),
            ]
        } else {
            vec![]
        })
    }

    pub fn focus_screen(&mut self, sel: &Selector<'_, Screen>) -> Vec<EventAction> {
        match self.inner.focus(sel) {
            Some((true, focused)) => vec![
                EventAction::SetActiveWorkspace(focused.wix),
                EventAction::RunHook(HookName::ScreenChange),
            ],
            _ => vec![],
        }
    }

    pub fn cycle_screen<S>(&mut self, direction: Direction, state: &S) -> Result<Vec<EventAction>>
    where
        S: XState,
    {
        if !self.inner.would_wrap(direction) {
            self.inner.cycle_focus(direction);
            let focused = self.inner.focused_unchecked();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::xconnection::*;

    fn raw_screens() -> Vec<Screen> {
        vec![
            Screen::new(Region::new(0, 0, 1366, 768), 0),
            Screen::new(Region::new(1366, 0, 1366, 768), 1),
        ]
    }

    #[test]
    fn update_known_screens_generates_events_when_there_is_a_change() {
        let mut s = Screens::new(10, true);
        let conn = MockXConn::new(raw_screens(), vec![], vec![]);
        let events = s.update_known_screens(&conn, 10).unwrap();

        assert_eq!(
            events,
            vec![
                EventAction::LayoutVisible,
                EventAction::RunHook(HookName::ScreenUpdated),
            ]
        )
    }

    #[test]
    fn update_known_screens_doesnt_generates_events_when_screens_are_unchanged() {
        let mut s = Screens::new(10, true);
        let conn = MockXConn::new(raw_screens(), vec![], vec![]);
        s.update_known_screens(&conn, 10).unwrap();
        let events = s.update_known_screens(&conn, 10).unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn changing_focus_generates_event_actions() {
        let mut s = Screens::new(10, true);
        let conn = MockXConn::new(raw_screens(), vec![], vec![]);
        s.update_known_screens(&conn, 10).unwrap();
        let events = s.focus_screen(&Selector::Index(1));

        assert_eq!(
            events,
            vec![
                EventAction::SetActiveWorkspace(1),
                EventAction::RunHook(HookName::ScreenChange)
            ]
        )
    }

    #[test]
    fn changing_focus_only_generates_event_actions_on_change() {
        let mut s = Screens::new(10, true);
        let conn = MockXConn::new(raw_screens(), vec![], vec![]);
        s.update_known_screens(&conn, 10).unwrap();
        let events = s.focus_screen(&Selector::Index(0));

        assert!(events.is_empty());
    }

    #[test]
    fn cycle_screen_generates_event_actions() {
        let mut s = Screens::new(10, true);
        let conn = MockXConn::new(raw_screens(), vec![], vec![]);
        s.update_known_screens(&conn, 10).unwrap();
        let events = s.cycle_screen(Direction::Forward, &conn).unwrap();

        assert_eq!(
            events,
            vec![
                EventAction::SetActiveWorkspace(1),
                EventAction::RunHook(HookName::ScreenChange)
            ]
        )
    }

    #[test]
    fn cycle_screen_does_not_generate_event_actions_when_unable_to_cycle() {
        let mut s = Screens::new(10, true);
        let conn = MockXConn::new(raw_screens(), vec![], vec![]);
        s.update_known_screens(&conn, 10).unwrap();
        let events = s.cycle_screen(Direction::Backward, &conn);

        assert!(events.unwrap().is_empty())
    }

    fn test_screens(h: u32, top_bar: bool) -> Vec<Screen> {
        let regions = &[
            Region::new(0, 0, 1000, 800),
            Region::new(1000, 0, 1400, 900),
        ];
        regions
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                let mut s = Screen::new(r, i);
                s.update_effective_region(h, top_bar);
                s
            })
            .collect()
    }

    struct OutputsXConn(Vec<Screen>);

    impl StubXAtomQuerier for OutputsXConn {}
    impl StubXState for OutputsXConn {
        fn mock_current_screens(&self) -> crate::core::xconnection::Result<Vec<Screen>> {
            Ok(self.0.clone())
        }
    }

    test_cases! {
        update_known_screens;
        args: (current: Vec<usize>, n_workspaces: usize, expected: Vec<usize>);

        case: unchanged => (vec![0, 1], 10, vec![0, 1]);
        case: non_default_workspaces => (vec![5, 7], 10, vec![5, 7]);
        case: new_take_first_available_0 => (vec![0], 10, vec![0, 1]);
        case: new_take_first_available_2 => (vec![2], 10, vec![2, 0]);
        case: fewer_retains_from_left => (vec![3, 5, 9], 10, vec![3, 5]);
        case: more_truncates => (vec![0], 1, vec![0]);

        body: {
            let (bar_height, top_bar) = (10, true);
            let screens = test_screens(bar_height, top_bar);
            let conn = OutputsXConn(screens);
            let mut s = Screens {
                inner: Ring::new(
                    current.into_iter().map(|wix|
                        Screen::new(Region::new(0, 0, 0, 0), wix)
                    ).collect()
                ),
                bar_height,
                top_bar
            };

            s.update_known_screens(&conn, n_workspaces).unwrap();
            let focused: Vec<usize> = s.inner.iter().map(|s| s.wix).collect();

            assert_eq!(focused, expected);
        }
    }
}
