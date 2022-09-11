use crate::{
    stack,
    state::{Layout, Position, Rect, Screen, Stack, Workspace},
    Error, Result,
};
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

// TODO: Should current & visible be wrapped up as another Stack?
/// The side-effect free internal state representation of the window manager.
#[derive(Debug, PartialEq, Eq)]
pub struct State<C, D>
where
    C: Clone + PartialEq,
{
    current: Screen<C, D>,             // Currently focused workspace
    visible: LinkedList<Screen<C, D>>, // Non-focused workspaces, visible in xinerama
    hidden: LinkedList<Workspace<C>>,  // Workspaces not currently on any screen
    floating: HashMap<u32, Rect>,      // Floating windows
}

impl<C, D> State<C, D>
where
    C: Clone + PartialEq,
{
    /// Create a new [StackSet] of empty stacks with the given workspace names.
    ///
    /// # Errors
    /// This method will error if there are not enough workspaces to cover the
    /// attached screens or if no screens are attached.
    pub fn try_new<I, J, T>(layout: Layout, ws_tags: I, screen_details: J) -> Result<Self>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
        J: IntoIterator<Item = D>,
    {
        let workspaces: Vec<Workspace<C>> = ws_tags
            .into_iter()
            .map(|tag| Workspace::empty(tag, layout.clone()))
            .collect();

        let screen_details: Vec<D> = screen_details.into_iter().collect();

        Self::try_new_concrete(workspaces, screen_details)
    }

    fn try_new_concrete(mut workspaces: Vec<Workspace<C>>, screen_details: Vec<D>) -> Result<Self> {
        // TODO: Enforce unique

        match (workspaces.len(), screen_details.len()) {
            (_, 0) => return Err(Error::NoScreens),
            (n_ws, n_screens) if n_ws < n_screens => {
                return Err(Error::InsufficientWorkspaces { n_ws, n_screens })
            }
            _ => (),
        }

        let hidden: LinkedList<Workspace<C>> = workspaces
            .split_off(screen_details.len())
            .into_iter()
            .collect();

        let mut visible: LinkedList<Screen<C, D>> = workspaces
            .into_iter()
            .zip(screen_details)
            .enumerate()
            .map(|(index, (workspace, screen_detail))| Screen {
                workspace,
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
        if let Some(mut s) = pop_where!(self, visible, |s: &Screen<C, D>| s.workspace.tag == tag) {
            std::mem::swap(&mut s, &mut self.current);
            self.visible.push_back(s);
            return;
        }

        // If the tag is hidden then it gets moved to the current screen
        if let Some(mut w) = pop_where!(self, hidden, |w: &Workspace<C>| w.tag == tag) {
            std::mem::swap(&mut w, &mut self.current.workspace);
            self.hidden.push_back(w);
        }

        // If nothing matched by this point then the requested tag is unknown
        // so there is nothing for us to do
    }

    pub fn focus_client(&mut self, client: &C) {
        if self.current_client() == Some(client) {
            return; // already focused
        }

        let tag = match self.tag_for_client(client) {
            Some(tag) => tag.to_string(),
            None => return, // unknown client
        };

        self.focus_tag(&tag);

        while self.current_client() != Some(client) {
            self.focus_up()
        }
    }

    /// Iterate over each [Screen] in this [State] in an arbitrary order.
    pub fn iter_screens(&self) -> impl Iterator<Item = &Screen<C, D>> {
        std::iter::once(&self.current).chain(self.visible.iter())
    }

    /// Mutably iterate over each [Screen] in this [State] in an arbitrary order.
    pub fn iter_screens_mut(&mut self) -> impl Iterator<Item = &mut Screen<C, D>> {
        std::iter::once(&mut self.current).chain(self.visible.iter_mut())
    }

    /// Iterate over each [Workspace] in this [State] in an arbitrary order.
    pub fn iter_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        std::iter::once(&self.current.workspace)
            .chain(self.visible.iter().map(|s| &s.workspace))
            .chain(self.hidden.iter())
    }

    /// Mutably iterate over each [Workspace] in this [State] in an arbitrary order.
    pub fn iter_workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        std::iter::once(&mut self.current.workspace)
            .chain(self.visible.iter_mut().map(|s| &mut s.workspace))
            .chain(self.hidden.iter_mut())
    }

    /// Iterate over each client in this [State] in an arbitrary order.
    pub fn iter_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_workspaces()
            .flat_map(|w| w.stack.iter().map(|s| s.iter()).flatten())
    }

    /// Iterate over each client in this [State] in an arbitrary order.
    pub fn iter_clients_mut(&mut self) -> impl Iterator<Item = &mut C> {
        self.iter_workspaces_mut()
            .flat_map(|w| w.stack.iter_mut().map(|s| s.iter_mut()).flatten())
    }

    /// Find the tag of the [Workspace] currently displayed on [Screen] `index`.
    ///
    /// Returns [None] if the index is out of bounds
    pub fn tag_for_screen(&self, index: usize) -> Option<&str> {
        self.iter_screens()
            .find(|s| s.index == index)
            .map(|s| s.workspace.tag.as_str())
    }

    /// Find the tag of the [Workspace] containing a given client.
    /// Returns Some(tag) if the client is known otherwise None.
    pub fn tag_for_client(&self, client: &C) -> Option<&str> {
        self.iter_workspaces()
            .find(|w| {
                w.stack
                    .as_ref()
                    .map(|s| s.iter().any(|elem| elem == client))
                    .unwrap_or(false)
            })
            .map(|w| w.tag.as_str())
    }

    /// Returns `true` if the [State] contains an element equal to the given value.
    pub fn contains(&self, client: &C) -> bool {
        self.tag_for_client(client).is_some()
    }
    /// Extract a reference to the focused element of the current [Stack]
    pub fn current_client(&self) -> Option<&C> {
        self.current.workspace.stack.as_ref().map(|s| s.head())
    }

    /// Get a reference to the current [Stack] if there is one
    pub fn current_stack(&self) -> Option<&Stack<C>> {
        self.current.workspace.stack.as_ref()
    }

    /// The `tag` of the current [Workspace]
    pub fn current_tag(&self) -> &str {
        &self.current.workspace.tag
    }

    /// If the current [Stack] is [None], return `default` otherwise
    /// apply the function to it to generate a value
    pub fn with<T, F>(&self, default: T, f: F) -> T
    where
        F: Fn(&Stack<C>) -> T,
    {
        self.current_stack().map(f).unwrap_or_else(|| default)
    }

    /// Apply a function to modify the current [Stack] if there is one
    /// or compute and inject a default value if it is currently [None]
    pub fn modify<F>(&mut self, f: F)
    where
        F: FnOnce(Option<Stack<C>>) -> Option<Stack<C>>,
    {
        let current_stack = self.current.workspace.stack.take();
        self.current.workspace.stack = f(current_stack);
    }

    /// Apply a function to modify the current [Stack] if it is non-empty
    /// without allowing for emptying it entirely.
    pub fn modify_occupied<F>(&mut self, f: F)
    where
        F: FnOnce(Stack<C>) -> Stack<C>,
    {
        self.modify(|s| s.map(f))
    }

    /// Insert the given client to the current [Stack] in a default [Position].
    pub fn insert(&mut self, client: C) {
        self.insert_at(Position::default(), client)
    }

    /// Insert the given client to the current [Stack] at the requested [Position].
    /// If the client is already present somewhere in the [State] the state is unmodified.
    pub fn insert_at(&mut self, pos: Position, client: C) {
        if self.contains(&client) {
            return;
        }

        self.modify(|current_stack| match current_stack {
            Some(mut s) => {
                s.insert_at(pos, client);
                Some(s)
            }
            None => Some(stack!(client)),
        })
    }
}

macro_rules! defer_to_current_stack {
    ($(
        $(#[$doc_str:meta])*
        $method:ident
    ),+) => {
        impl<C, D> State<C, D>
        where
            C: Clone + PartialEq
        {
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

    fn test_state(n_tags: usize, n: usize) -> State<u8, u8> {
        let tags = (1..=n_tags).map(|n| n.to_string());

        State::try_new(Layout::default(), tags, vec![0; n]).unwrap()
    }

    fn test_state_with_stacks(wss: Vec<Stack<u8>>, n: usize) -> State<u8, u8> {
        let workspaces: Vec<Workspace<u8>> = wss
            .into_iter()
            .enumerate()
            .map(|(i, s)| Workspace::with_stack((i + 1).to_string(), Layout::default(), s))
            .collect();

        State::try_new_concrete(workspaces, vec![0; n]).unwrap()
    }

    #[test_case("1", &["2", "3"]; "current focused workspace")]
    #[test_case("2", &["3", "1"]; "visible on other screen")]
    #[test_case("3", &["2", "1"]; "currently hidden")]
    #[test]
    fn focus_tag_sets_correct_visible_workspaces(target: &str, vis: &[&str]) {
        let mut s = test_state(5, 3);

        s.focus_tag(target);

        let visible_tags: Vec<&str> = s.visible.iter().map(|s| s.workspace.tag.as_ref()).collect();

        assert_eq!(s.current.workspace.tag, target);
        assert_eq!(visible_tags, vis);
    }

    #[test_case(0, Some("1"), Some("3"); "initial focus")]
    #[test_case(1, Some("2"), Some("2"); "other screen")]
    #[test_case(2, None, None; "out of bounds")]
    #[test]
    fn tag_for_screen_works(index: usize, before: Option<&str>, after: Option<&str>) {
        let mut s = test_state(5, 2);

        assert_eq!(s.tag_for_screen(index), before);
        s.focus_tag("3");
        assert_eq!(s.tag_for_screen(index), after);
    }

    #[test_case(5, Some("1"); "in down")]
    #[test_case(6, Some("2"); "focus")]
    #[test_case(9, Some("3"); "in up")]
    #[test_case(42, None; "unknown")]
    #[test]
    fn tag_for_client_works(client: u8, expected: Option<&str>) {
        let s = test_state_with_stacks(
            vec![
                stack!([1, 2], 3, [4, 5]),
                stack!(6, [7, 8]),
                stack!([9], 10),
            ],
            1,
        );

        assert_eq!(s.tag_for_client(&client), expected);
    }

    #[test]
    fn insert_pushes_to_current_stack() {
        let mut s = test_state(5, 2);

        assert!(s.current_stack().is_none());
        s.insert(2);
        assert_eq!(s.current_client(), Some(&2));
    }
}
