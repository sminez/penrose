use crate::{Error, Layout, Rect, Result, Screen, ScreenDetail, Workspace};
use std::collections::{HashMap, LinkedList};

// Helper for popping from the middle of a linked list
macro_rules! pop_where {
    ($self:ident, $lst:ident, $pred:ident) => {{
        let mut placeholder = LinkedList::default();
        std::mem::swap(&mut $self.$lst, &mut placeholder);

        let mut remaining = LinkedList::default();
        let mut popped = None;

        for item in placeholder.into_iter() {
            if $pred(&item) {
                popped = Some(item);
            } else {
                remaining.push_back(item);
            }
        }

        std::mem::swap(&mut $self.$lst, &mut remaining);

        popped
    }};
}

#[derive(Debug, PartialEq, Eq)]
pub struct State {
    current: Screen,               // Currently focused workspace
    visible: LinkedList<Screen>,   // Non-focused workspaces, visible in xinerama
    hidden: LinkedList<Workspace>, // Workspaces not currently on any screen
    floating: HashMap<u32, Rect>,  // Floating windows
}

impl State {
    /// Create a new [StackSet] of empty stacks with the given workspace names.
    ///
    /// # Errors
    /// This method will error if there are not enough workspaces to cover the
    /// attached screens or if no screens are attached.
    pub fn try_new<T>(
        layout: Layout,
        ws_names: Vec<T>,
        screen_details: Vec<ScreenDetail>,
    ) -> Result<Self>
    where
        T: Into<String>,
    {
        match (ws_names.len(), screen_details.len()) {
            (n_ws, n_screens) if n_ws < n_screens => {
                return Err(Error::InsufficientWorkspaces { n_ws, n_screens })
            }
            (_, 0) => return Err(Error::NoScreens),
            _ => (),
        }

        // TODO: Enforce unique
        let mut ws_names: Vec<String> = ws_names.into_iter().map(|w| w.into()).collect();

        let ws = |tag| Workspace::empty(tag, layout.clone());

        let hidden = ws_names
            .split_off(screen_details.len())
            .into_iter()
            .map(ws)
            .collect();

        let mut visible: LinkedList<Screen> = ws_names
            .into_iter()
            .zip(screen_details)
            .enumerate()
            .map(|(index, (tag, screen_detail))| Screen {
                workspace: ws(tag),
                index,
                screen_detail,
            })
            .collect();

        let current = visible.pop_front().expect("to have at least one screen");

        Ok(Self {
            current,
            visible,
            hidden,
            floating: HashMap::new(),
        })
    }

    // TODO: should this return an error for an unmatched tag?
    /// Set focus to the [Workspace] with the specified tag.
    ///
    /// If there is no matching workspace then the [State] is unmodified.
    /// If the [Workspace] is currently visible it becomes the active [Screen],
    /// otherwise the workspace replaces whatever was on the active screen.
    pub fn focus_tag(&mut self, tag: &str) {
        // If the tag is already focused then there's nothing to do
        if tag == &self.current.workspace.tag {
            return;
        }

        // If the tag is visible on another screen, focus moves to that screen
        let matching_screen = |s: &Screen| s.workspace.tag == tag;

        if let Some(mut s) = pop_where!(self, visible, matching_screen) {
            std::mem::swap(&mut s, &mut self.current);
            self.visible.push_back(s);
        }

        // If the tag is hidden then it gets moved to the current screen
        let matching_ws = |w: &Workspace| w.tag == tag;

        if let Some(mut w) = pop_where!(self, hidden, matching_ws) {
            std::mem::swap(&mut w, &mut self.current.workspace);
            self.hidden.push_back(w);
        }

        // If nothing matched by this point then the requested tag is unknown
        // so there is nothing for us to do
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    fn test_state(tags: Vec<&str>) -> State {
        State::try_new(
            Layout::default(),
            tags,
            vec![ScreenDetail::default(); 3],
        )
        .unwrap()
    }

    #[test_case("1", &["2", "3"]; "current focus")]
    #[test_case("2", &["3", "1"]; "visible on other screen")]
    #[test_case("3", &["2", "1"]; "hidden")]
    #[test]
    fn focus_tag_sets_correct_visible_workspaces(target: &str, vis: &[&str]) {
        let mut s = test_state(vec!["1", "2", "3", "4", "5"]);

        s.focus_tag(target);

        let visible_tags: Vec<&str> = s.visible.iter().map(|s| s.workspace.tag.as_ref()).collect();

        assert_eq!(s.current.workspace.tag, target);
        assert_eq!(visible_tags, vis);
    }
}
