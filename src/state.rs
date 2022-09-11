use crate::{Client, Error, Layout, Rect, Result, Screen, ScreenDetail, Stack, Workspace};
use std::collections::{HashMap, LinkedList};

// Helper for popping from the middle of a linked list
macro_rules! pop_where {
    ($self:ident, $lst:ident, $($pred:tt)+) => {{
        let mut placeholder = LinkedList::default();
        std::mem::swap(&mut $self.$lst, &mut placeholder);

        let mut remaining = LinkedList::default();
        let mut popped = None;
        let pred = $($pred)+;

        for item in placeholder.into_iter() {
            if pred(&item) {
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
        if let Some(mut s) = pop_where!(self, visible, |s: &Screen| s.workspace.tag == tag) {
            std::mem::swap(&mut s, &mut self.current);
            self.visible.push_back(s);
            return;
        }

        // If the tag is hidden then it gets moved to the current screen
        if let Some(mut w) = pop_where!(self, hidden, |w: &Workspace| w.tag == tag) {
            std::mem::swap(&mut w, &mut self.current.workspace);
            self.hidden.push_back(w);
        }

        // If nothing matched by this point then the requested tag is unknown
        // so there is nothing for us to do
    }

    // Iterate over all screens:
    //   Current -> visible
    fn iter_screens(&self) -> impl Iterator<Item = &Screen> {
        std::iter::once(&self.current).chain(self.visible.iter())
    }

    // Iterate over all workspaces:
    //   Current -> visible -> hidden
    // fn iter_workspaces(&self) -> impl Iterator<Item = &Workspace> {
    //     std::iter::once(&self.current.workspace)
    //         .chain(self.visible.iter().map(|s| &s.workspace))
    //         .chain(self.hidden.iter())
    // }

    /// Find the tag of the [Workspace] currently displayed on [Screen] `index`.
    ///
    /// Returns [None] if the index is out of bounds
    pub fn tag_for_screen(&self, index: usize) -> Option<&str> {
        self.iter_screens()
            .find(|s| s.index == index)
            .map(|s| s.workspace.tag.as_str())
    }

    /// Extract a reference to the focused element of the current [Stack]
    pub fn peek(&self) -> Option<&Client> {
        self.current.workspace.stack.as_ref().map(|s| s.head())
    }

    /// Get a reference to the current [Stack] if there is one
    pub fn current_stack(&self) -> Option<&Stack<Client>> {
        self.current.workspace.stack.as_ref()
    }

    // If the current [Stack] is [None], return `default` otherwise
    // apply the function to it to generate a value
    fn with<T, F>(&self, default: T, f: F) -> T
    where
        F: Fn(&Stack<Client>) -> T,
    {
        self.current_stack().map(f).unwrap_or_else(|| default)
    }

    // Apply a function to modify the current [Stack] if there is one
    // or inject a default value if it is currently [None]
    fn modify<F>(&mut self, default: Option<Stack<Client>>, f: F)
    where
        F: Fn(Stack<Client>) -> Option<Stack<Client>>,
    {
        let current_stack = self.current.workspace.stack.take();

        self.current.workspace.stack = match current_stack {
            Some(stack) => f(stack),
            None => default,
        };
    }

    // Apply a function to modify the current [Stack] if it is non-empty
    // without allowing for emptying it
    fn modify_occupied<F>(&mut self, f: F)
    where
        F: Fn(Stack<Client>) -> Stack<Client>,
    {
        self.modify(None, |s| Some(f(s)))
    }
}

macro_rules! defer_to_current_stack {
    ($(
        $(#[$doc_str:meta])*
        $method:ident
    ),+) => {
        impl State {
            $(
                pub fn $method(&mut self) {
                    if let Some(ref mut stack) = self.current.workspace.stack {
                        stack.$method()
                    }
                }
            )+
        }
    }
}

defer_to_current_stack!(
    /// Move focus from the current element up the [Stack], wrapping to
    /// the bottom if focus is already at the top.
    /// This is a no-op if the current stack is empty.
    focus_up,
    /// Move focus from the current element down the [Stack], wrapping to
    /// the top if focus is already at the bottom.
    /// This is a no-op if the current stack is empty.
    focus_down,
    /// Swap the position of the focused element with one above it.
    /// The currently focused element is maintained by this operation.
    /// This is a no-op if the current stack is empty.
    swap_up,
    /// Swap the position of the focused element with one below it.
    /// The currently focused element is maintained by this operation.
    /// This is a no-op if the current stack is empty.
    swap_down,
    /// Rotate all elements of the stack forward, wrapping from top to bottom.
    /// The currently focused position in the stack is maintained by this operation.
    /// This is a no-op if the current stack is empty.
    rotate_up,
    /// Rotate all elements of the stack back, wrapping from bottom to top.
    /// The currently focused position in the stack is maintained by this operation.
    /// This is a no-op if the current stack is empty.
    rotate_down
);

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    fn test_state(tags: Vec<&str>, n: usize) -> State {
        State::try_new(Layout::default(), tags, vec![ScreenDetail::default(); n]).unwrap()
    }

    #[test_case("1", &["2", "3"]; "current focused workspace")]
    #[test_case("2", &["3", "1"]; "visible on other screen")]
    #[test_case("3", &["2", "1"]; "currently hidden")]
    #[test]
    fn focus_tag_sets_correct_visible_workspaces(target: &str, vis: &[&str]) {
        let mut s = test_state(vec!["1", "2", "3", "4", "5"], 3);

        s.focus_tag(target);

        let visible_tags: Vec<&str> = s.visible.iter().map(|s| s.workspace.tag.as_ref()).collect();

        assert_eq!(s.current.workspace.tag, target);
        assert_eq!(visible_tags, vis);
    }

    #[test]
    fn tag_for_screen_works() {
        let mut s = test_state(vec!["1", "2", "3", "4", "5"], 2);

        assert_eq!(s.tag_for_screen(0), Some("1"));
        assert_eq!(s.tag_for_screen(1), Some("2"));
        assert_eq!(s.tag_for_screen(2), None);

        s.focus_tag("3");

        assert_eq!(s.tag_for_screen(0), Some("3"));
        assert_eq!(s.tag_for_screen(1), Some("2"));
        assert_eq!(s.tag_for_screen(2), None);
    }
}
