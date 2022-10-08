use crate::{
    geometry::Rect,
    layout::LayoutStack,
    stack,
    stack_set::{Position, Screen, Stack, Workspace},
    Error, Result,
};
use std::{
    collections::{HashMap, HashSet, LinkedList},
    hash::Hash,
    mem::{swap, take},
};

// Helper for popping from the middle of a linked list
#[doc(hidden)]
#[macro_export]
macro_rules! pop_where {
    ($self:ident, $lst:ident, $($pred:tt)+) => {{
        let placeholder = take(&mut $self.$lst);

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

        swap(&mut $self.$lst, &mut remaining);

        popped
    }};
}

// TODO: Should current & visible be wrapped up as another Stack?
/// The side-effect free internal state representation of the window manager.
#[derive(Debug, Clone)]
pub struct StackSet<C>
where
    C: Clone + PartialEq + Eq + Hash,
{
    pub(crate) current: Screen<C>, // Currently focused workspace
    pub(crate) visible: LinkedList<Screen<C>>, // Non-focused workspaces, visible in xinerama
    pub(crate) hidden: LinkedList<Workspace<C>>, // Workspaces not currently on any screen
    pub(crate) floating: HashMap<C, Rect>, // Floating windows
}

impl<C> StackSet<C>
where
    C: Clone + PartialEq + Eq + Hash,
{
    /// Create a new [StackSet] of empty stacks with the given workspace names.
    ///
    /// # Errors
    /// This method will error if there are not enough workspaces to cover the
    /// attached screens or if no screens are attached.
    pub fn try_new<I, J, T>(layouts: LayoutStack, ws_tags: I, screen_details: J) -> Result<Self>
    where
        T: Into<String>,
        I: IntoIterator<Item = T>,
        J: IntoIterator<Item = Rect>,
    {
        let workspaces: Vec<Workspace<C>> = ws_tags
            .into_iter()
            .enumerate()
            .map(|(i, tag)| Workspace::new(i, tag, layouts.clone(), None))
            .collect();

        let screen_details: Vec<Rect> = screen_details.into_iter().collect();

        Self::try_new_concrete(workspaces, screen_details)
    }

    fn try_new_concrete(
        mut workspaces: Vec<Workspace<C>>,
        screen_details: Vec<Rect>,
    ) -> Result<Self> {
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

        let mut visible: LinkedList<Screen<C>> = workspaces
            .into_iter()
            .zip(screen_details)
            .enumerate()
            .map(|(index, (workspace, r))| Screen {
                workspace,
                index,
                r,
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
    /// If there is no matching workspace then the [StackSet] is unmodified.
    /// If the [Workspace] is currently visible it becomes the active [Screen],
    /// otherwise the workspace replaces whatever was on the active screen.
    pub fn focus_tag(&mut self, tag: &str) {
        // If the tag is already focused then there's nothing to do
        if tag == &self.current.workspace.tag {
            return;
        }

        // If the tag is visible on another screen, focus moves to that screen
        if let Some(mut s) = pop_where!(self, visible, |s: &Screen<C>| s.workspace.tag == tag) {
            swap(&mut s, &mut self.current);
            self.visible.push_back(s);
            return;
        }

        // If the tag is hidden then it gets moved to the current screen
        if let Some(mut w) = pop_where!(self, hidden, |w: &Workspace<C>| w.tag == tag) {
            swap(&mut w, &mut self.current.workspace);
            self.hidden.push_back(w);
        }

        // If nothing matched by this point then the requested tag is unknown
        // so there is nothing for us to do
    }

    /// Focus the given client and set its [Workspace] as current (see
    /// focus_tag).
    ///
    /// If the client is unknown then this is a no-op.
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

    /// Insert the given client to the current [Stack] in a default [Position].
    pub fn insert(&mut self, client: C) {
        self.insert_at(Position::default(), client)
    }

    /// Insert the given client to the current [Stack] at the requested [Position].
    /// If the client is already present somewhere in the [StackSet] the stack_set is unmodified.
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

    /// Record a known client as floating, giving its preferred screen position.
    ///
    /// # Errors
    /// This method with return [Error::UnknownClient] if the given client is
    /// not already managed in this stack_set.
    pub fn float(&mut self, client: C, r: Rect) -> Result<()> {
        if !self.contains(&client) {
            return Err(Error::UnknownClient);
        }
        self.float_unchecked(client, r);

        Ok(())
    }

    pub(crate) fn float_unchecked(&mut self, client: C, r: Rect) {
        self.floating.insert(client, r);
    }

    /// Clear the floating status of a client, returning its previous preferred
    /// screen position if the client was known, otherwise `None`.
    pub fn sink(&mut self, client: &C) -> Option<Rect> {
        self.floating.remove(client)
    }

    /// Delete a client from this [StackSet].
    pub fn remove_client(&mut self, client: &C) -> Option<C> {
        self.sink(client); // Clear any floating information we might have

        self.iter_workspaces_mut()
            .map(|w| w.remove(client))
            .find(|opt| opt.is_some())?
    }

    /// Move the focused client of the current [Workspace] to the focused position
    /// of the workspace matching the provided `tag`.
    pub fn move_focused_to_tag(&mut self, tag: &str) {
        if self.current_tag() == tag || !self.contains_tag(tag) {
            return;
        }

        let c = match self.current.workspace.remove_focused() {
            None => return,
            Some(c) => c,
        };

        self.insert_as_focus_for(tag, c)
    }

    /// Move the given client to the focused position of the [Workspace] matching
    /// the provided `tag`. If the client is already on the target workspace it is
    /// moved to the focused position.
    pub fn move_client_to_tag(&mut self, client: &C, tag: &str) {
        if !self.contains_tag(tag) {
            return;
        }

        let c = match self.remove_client(client) {
            None => return,
            Some(c) => c,
        };

        self.insert_as_focus_for(tag, c)
    }

    fn insert_as_focus_for(&mut self, tag: &str, c: C) {
        self.modify_workspace(tag, |w| {
            w.stack = Some(match take(&mut w.stack) {
                None => stack!(c),
                Some(mut s) => {
                    s.insert(c);
                    s
                }
            });
        });
    }

    fn contains_tag(&self, tag: &str) -> bool {
        self.iter_workspaces().any(|w| w.tag == tag)
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

    /// Find the tag of the [Workspace] with the given NetWmDesktop ID.
    pub fn tag_for_workspace_id(&self, id: usize) -> Option<String> {
        self.iter_workspaces()
            .find(|w| w.id == id)
            .map(|w| w.tag.clone())
    }

    /// Returns `true` if the [StackSet] contains an element equal to the given value.
    pub fn contains(&self, client: &C) -> bool {
        self.iter_clients().any(|c| c == client)
    }

    /// Extract a reference to the focused element of the current [Stack]
    pub fn current_client(&self) -> Option<&C> {
        self.current.workspace.stack.as_ref().map(|s| &s.focus)
    }

    /// Get a reference to the current [Stack] if there is one
    pub fn current_stack(&self) -> Option<&Stack<C>> {
        self.current.workspace.stack.as_ref()
    }

    /// The `tag` of the current [Workspace]
    pub fn current_tag(&self) -> &str {
        &self.current.workspace.tag
    }

    /// A reference to the [Workspace] with a tag of `tag` if there is one
    pub fn workspace(&self, tag: &str) -> Option<&Workspace<C>> {
        self.iter_workspaces().find(|w| w.tag == tag)
    }

    /// A mutable reference to the [Workspace] with a tag of `tag` if there is one
    pub fn workspace_mut(&mut self, tag: &str) -> Option<&mut Workspace<C>> {
        self.iter_workspaces_mut().find(|w| w.tag == tag)
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
        self.current.workspace.stack = f(take(&mut self.current.workspace.stack));
    }

    /// Apply a function to modify the current [Stack] if it is non-empty
    /// without allowing for emptying it entirely.
    pub fn modify_occupied<F>(&mut self, f: F)
    where
        F: FnOnce(Stack<C>) -> Stack<C>,
    {
        self.modify(|s| s.map(f))
    }

    fn modify_workspace<F>(&mut self, tag: &str, f: F)
    where
        F: FnOnce(&mut Workspace<C>),
    {
        self.iter_workspaces_mut().find(|w| w.tag == tag).map(f);
    }

    /// Iterate over each [Screen] in this [StackSet] in an arbitrary order.
    pub fn iter_screens(&self) -> impl Iterator<Item = &Screen<C>> {
        std::iter::once(&self.current).chain(self.visible.iter())
    }

    /// Mutably iterate over each [Screen] in this [StackSet] in an arbitrary order.
    pub fn iter_screens_mut(&mut self) -> impl Iterator<Item = &mut Screen<C>> {
        std::iter::once(&mut self.current).chain(self.visible.iter_mut())
    }

    /// Iterate over each [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        std::iter::once(&self.current.workspace)
            .chain(self.visible.iter().map(|s| &s.workspace))
            .chain(self.hidden.iter())
    }

    /// Iterate over the currently visible [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_visible_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        std::iter::once(&self.current.workspace).chain(self.visible.iter().map(|s| &s.workspace))
    }

    /// Iterate over the currently hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_hidden_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.hidden.iter()
    }

    /// Iterate over the currently hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_hidden_workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        self.hidden.iter_mut()
    }

    /// Mutably iterate over each [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        std::iter::once(&mut self.current.workspace)
            .chain(self.visible.iter_mut().map(|s| &mut s.workspace))
            .chain(self.hidden.iter_mut())
    }

    /// Iterate over each client in this [StackSet] in an arbitrary order.
    pub fn iter_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_workspaces()
            .flat_map(|w| w.stack.iter().map(|s| s.iter()).flatten())
    }

    /// Iterate over the currently visible clients in this [StackSet] in an arbitrary order.
    pub fn iter_visible_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_visible_workspaces()
            .flat_map(|w| w.stack.iter().map(|s| s.iter()).flatten())
    }

    /// Iterate over the currently hidden clients in this [StackSet] in an arbitrary order.
    pub fn iter_hidden_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_hidden_workspaces()
            .flat_map(|w| w.stack.iter().map(|s| s.iter()).flatten())
    }

    /// Iterate over each client in this [StackSet] in an arbitrary order.
    pub fn iter_clients_mut(&mut self) -> impl Iterator<Item = &mut C> {
        self.iter_workspaces_mut()
            .flat_map(|w| w.stack.iter_mut().map(|s| s.iter_mut()).flatten())
    }
}

impl<C> StackSet<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) fn snapshot(&self) -> Snapshot<C> {
        Snapshot {
            focus: self.current_client().copied(),
            visible_clients: self.iter_visible_clients().cloned().collect(),
            hidden_clients: self.iter_hidden_clients().cloned().collect(),
            visible_tags: self
                .iter_visible_workspaces()
                .map(|w| w.tag.clone())
                .collect(),
        }
    }
}

macro_rules! defer_to_current_stack {
    ($(
        $(#[$doc_str:meta])*
        $method:ident
    ),+) => {
        impl<C> StackSet<C>
        where
            C: Clone + PartialEq + Eq + Hash
        {
            $(
                pub fn $method(&mut self) {
                    if let Some(ref mut stack) = self.current.workspace.stack {
                        stack.$method();
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

pub(crate) struct Snapshot<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) focus: Option<C>,
    pub(crate) visible_clients: HashSet<C>,
    pub(crate) hidden_clients: HashSet<C>,
    pub(crate) visible_tags: HashSet<String>,
}

impl<C> Snapshot<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) fn all_clients(&self) -> impl Iterator<Item = &C> {
        self.visible_clients
            .iter()
            .chain(self.hidden_clients.iter())
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Diff<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) old_focus: Option<C>,
    pub(crate) new: Vec<C>,
    pub(crate) hidden: Vec<C>,
    pub(crate) visible: Vec<C>,
    pub(crate) withdrawn: Vec<C>,
    pub(crate) previous_visible_tags: HashSet<String>,
}

impl<C> Diff<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) fn from_raw(ss: Snapshot<C>, s: &StackSet<C>, positions: &[(C, Rect)]) -> Self {
        let new: Vec<C> = s
            .iter_clients()
            .filter(|&&c| !ss.all_clients().any(|&cc| cc == c))
            .cloned()
            .collect();

        let visible: Vec<C> = positions.iter().map(|&(client, _)| client).collect();

        let hidden = ss
            .visible_clients
            .iter()
            .chain(new.iter())
            .filter(|&c| !visible.contains(c))
            .copied()
            .collect();

        let withdrawn = ss
            .all_clients()
            .filter(|&&c| !s.iter_clients().any(|&cc| cc == c))
            .copied()
            .collect();

        let previous_visible_tags = s
            .iter_hidden_workspaces()
            .map(|ws| ws.tag.clone())
            .filter(|t| ss.visible_tags.contains(t))
            .collect();

        Self {
            old_focus: ss.focus,
            new,
            hidden,
            visible,
            withdrawn,
            previous_visible_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    pub fn test_stack_set(n_tags: usize, n: usize) -> StackSet<u8> {
        let tags = (1..=n_tags).map(|n| n.to_string());

        StackSet::try_new(LayoutStack::default(), tags, vec![Rect::default(); n]).unwrap()
    }

    pub fn test_stack_set_with_stacks(stacks: Vec<Option<Stack<u8>>>, n: usize) -> StackSet<u8> {
        let workspaces: Vec<Workspace<u8>> = stacks
            .into_iter()
            .enumerate()
            .map(|(i, s)| Workspace::new(i, (i + 1).to_string(), LayoutStack::default(), s))
            .collect();

        match StackSet::try_new_concrete(workspaces, vec![Rect::default(); n]) {
            Ok(s) => s,
            Err(e) => panic!("{e}"),
        }
    }

    #[test_case("1", &["2", "3"]; "current focused workspace")]
    #[test_case("2", &["3", "1"]; "visible on other screen")]
    #[test_case("3", &["2", "1"]; "currently hidden")]
    #[test]
    fn focus_tag_sets_correct_visible_workspaces(target: &str, vis: &[&str]) {
        let mut s = test_stack_set(5, 3);

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
        let mut s = test_stack_set(5, 2);

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
        let s = test_stack_set_with_stacks(
            vec![
                Some(stack!([1, 2], 3, [4, 5])),
                Some(stack!(6, [7, 8])),
                Some(stack!([9], 10)),
            ],
            1,
        );

        assert_eq!(s.tag_for_client(&client), expected);
    }

    #[test_case(None; "empty current stack")]
    #[test_case(Some(stack!(1)); "current stack with one element")]
    #[test_case(Some(stack!([2], 1)); "current stack with up")]
    #[test_case(Some(stack!(1, [3])); "current stack with down")]
    #[test_case(Some(stack!([2], 1, [3])); "current stack with up and down")]
    #[test]
    fn insert(stack: Option<Stack<u8>>) {
        let mut s = test_stack_set_with_stacks(vec![stack], 1);
        s.insert(42);

        assert!(s.contains(&42))
    }

    fn test_iter_stack_set() -> StackSet<u8> {
        test_stack_set_with_stacks(
            vec![
                Some(stack!(1)),
                Some(stack!([2], 3)),
                Some(stack!(4, [5])),
                None,
                Some(stack!([6], 7, [8])),
            ],
            3,
        )
    }

    #[test]
    fn iter_screens_returns_all_screens() {
        let s = test_iter_stack_set();
        let mut screen_indices: Vec<usize> = s.iter_screens().map(|s| s.index).collect();
        screen_indices.sort();

        assert_eq!(screen_indices, vec![0, 1, 2])
    }

    #[test]
    fn iter_screens_mut_returns_all_screens() {
        let mut s = test_iter_stack_set();
        let mut screen_indices: Vec<usize> = s.iter_screens_mut().map(|s| s.index).collect();
        screen_indices.sort();

        assert_eq!(screen_indices, vec![0, 1, 2])
    }

    #[test]
    fn iter_workspaces_returns_all_workspaces() {
        let s = test_iter_stack_set();
        let mut tags: Vec<&str> = s.iter_workspaces().map(|w| w.tag.as_str()).collect();
        tags.sort();

        assert_eq!(tags, vec!["1", "2", "3", "4", "5"])
    }

    #[test]
    fn iter_workspaces_mut_returns_all_workspaces() {
        let mut s = test_iter_stack_set();
        let mut tags: Vec<&str> = s.iter_workspaces_mut().map(|w| w.tag.as_str()).collect();
        tags.sort();

        assert_eq!(tags, vec!["1", "2", "3", "4", "5"])
    }

    #[test]
    fn iter_clients_returns_all_clients() {
        let s = test_iter_stack_set();
        let mut clients: Vec<u8> = s.iter_clients().map(|c| *c).collect();
        clients.sort();

        assert_eq!(clients, vec![1, 2, 3, 4, 5, 6, 7, 8])
    }

    #[test]
    fn iter_clients_mut_returns_all_clients() {
        let mut s = test_iter_stack_set();
        let mut clients: Vec<u8> = s.iter_clients_mut().map(|c| *c).collect();
        clients.sort();

        assert_eq!(clients, vec![1, 2, 3, 4, 5, 6, 7, 8])
    }

    #[test_case(stack!(1); "current stack with one element")]
    #[test_case(stack!([2], 1); "current stack with up")]
    #[test_case(stack!(1, [3]); "current stack with down")]
    #[test_case(stack!([2], 1, [3]); "current stack with up and down")]
    #[test]
    fn contains(stack: Stack<u8>) {
        let s = test_stack_set_with_stacks(vec![Some(stack)], 1);

        assert!(s.contains(&1))
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::{tests::test_stack_set_with_stacks, *};
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::collections::HashSet;

    impl Stack<u8> {
        pub fn try_from_arbitrary_vec(mut up: Vec<u8>, g: &mut Gen) -> Option<Self> {
            let focus = match up.len() {
                0 => return None,
                1 => return Some(stack!(up.remove(0))),
                _ => up.remove(0),
            };

            let split_at = usize::arbitrary(g) % (up.len());
            let down = up.split_off(split_at);

            Some(Self::new(up, focus, down))
        }
    }

    impl StackSet<u8> {
        fn minimal_unknown_client(&self) -> u8 {
            let mut c = 0;

            while self.contains(&c) {
                c += 1;
            }

            c
        }

        fn first_hidden_tag(&self) -> Option<String> {
            self.hidden.iter().map(|w| w.tag.clone()).next()
        }

        fn last_tag(&self) -> String {
            self.iter_workspaces()
                .last()
                .expect("at least one workspace")
                .tag
                .clone()
        }

        fn last_visible_client(&self) -> Option<&u8> {
            self.visible
                .back()
                .unwrap_or(&self.current)
                .workspace
                .stack
                .iter()
                .map(|s| s.iter())
                .flatten()
                .last()
        }
    }

    // For the tests below we only care about the stack structure not the elements themselves, so
    // we use `u8` as an easily defaultable focus if `Vec::arbitrary` gives us an empty vec.
    impl Arbitrary for StackSet<u8> {
        fn arbitrary(g: &mut Gen) -> Self {
            let n_stacks = usize::arbitrary(g) % 10;
            let mut stacks = Vec::with_capacity(n_stacks);

            let mut clients: Vec<u8> = HashSet::<u8>::arbitrary(g).into_iter().collect();

            for _ in 0..n_stacks {
                if clients.is_empty() {
                    stacks.push(None);
                    continue;
                }

                let split_at = usize::arbitrary(g) % (clients.len());
                let stack_clients = clients.split_off(split_at);
                stacks.push(Stack::try_from_arbitrary_vec(stack_clients, g));
            }

            stacks.push(Stack::try_from_arbitrary_vec(clients, g));

            let n_screens = if n_stacks == 0 {
                1
            } else {
                std::cmp::max(usize::arbitrary(g) % n_stacks, 1)
            };

            test_stack_set_with_stacks(stacks, n_screens)
        }
    }

    #[quickcheck]
    fn insert_pushes_to_current_stack(mut s: StackSet<u8>) -> bool {
        let new_focus = s.minimal_unknown_client();
        s.insert(new_focus);

        s.current_client() == Some(&new_focus)
    }

    #[quickcheck]
    fn focus_client_focused_the_enclosing_workspace(mut s: StackSet<u8>) -> bool {
        let target = match s.iter_clients().max() {
            Some(target) => target.clone(),
            None => return true, // nothing to focus
        };

        let expected = s
            .tag_for_client(&target)
            .expect("client is known so tag is Some")
            .to_owned();

        s.focus_client(&target);

        s.current_tag() == expected
    }

    #[quickcheck]
    fn move_focused_to_tag(mut s: StackSet<u8>) -> bool {
        let tag = s.last_tag();

        let c = match s.current_client() {
            Some(&c) => c,
            None => return true, // no focused client to move for this case
        };

        s.move_focused_to_tag(&tag);
        s.focus_tag(&tag);

        s.current_client() == Some(&c)
    }

    #[quickcheck]
    fn move_client_to_tag(mut s: StackSet<u8>) -> bool {
        let tag = s.last_tag();

        let c = match s.last_visible_client() {
            Some(&c) => c,
            None => return true, // no client to move for this case
        };

        s.move_client_to_tag(&c, &tag);
        s.focus_tag(&tag);

        s.current_client() == Some(&c)
    }

    fn is_empty_diff(diff: &Diff<u8>) -> bool {
        diff.new.is_empty()
            && diff.hidden.is_empty()
            && diff.withdrawn.is_empty()
            && diff.previous_visible_tags.is_empty()
    }

    #[quickcheck]
    fn diff_of_unchanged_stackset_is_empty(s: StackSet<u8>) -> bool {
        let ss = s.snapshot();
        let same_positions: Vec<_> = ss
            .visible_clients
            .iter()
            .map(|&c| (c, Rect::default()))
            .collect();
        let diff = Diff::from_raw(ss, &s, &same_positions);

        is_empty_diff(&diff)
    }

    #[quickcheck]
    fn adding_a_client_is_new_in_diff(mut s: StackSet<u8>) -> bool {
        let ss = s.snapshot();
        let new = s.minimal_unknown_client();

        s.insert(new);
        let diff = Diff::from_raw(ss, &s, &[]);

        diff.new.contains(&new)
    }

    // NOTE: Not checking that clients on the new workspace are visible as this is driven entirely by
    //       the positions returned by the Layout. In these tests, those are being specified manually
    //       so there is nothing to test.
    #[quickcheck]
    fn focusing_new_workspace_hides_old_clients_and_tag_in_diff(mut s: StackSet<u8>) -> bool {
        let ss = s.snapshot();
        let tag = match s.first_hidden_tag() {
            Some(t) => t,
            None => return true,
        };
        let prev_tag = s.current_tag().to_string();
        let clients_on_active: Vec<u8> = match s.current_stack() {
            Some(stack) => stack.iter().cloned().collect(),
            None => vec![],
        };

        s.focus_tag(&tag);
        let diff = Diff::from_raw(ss, &s, &[]);

        let focused_clients_now_hidden = clients_on_active.iter().all(|c| diff.hidden.contains(&c));
        let tag_now_hidden = diff.previous_visible_tags.contains(&prev_tag);

        focused_clients_now_hidden && tag_now_hidden
    }

    #[quickcheck]
    fn killing_focused_client_sets_withdrawn_and_hidden_in_diff(mut s: StackSet<u8>) -> bool {
        let ss = s.snapshot();

        let prev_focus = match s.current_client() {
            Some(&c) => c,
            None => return true, // nothing to remove
        };

        s.remove_client(&prev_focus);
        let diff = Diff::from_raw(ss, &s, &[]);

        diff.withdrawn.contains(&prev_focus) && diff.hidden.contains(&prev_focus)
    }

    #[quickcheck]
    fn moving_client_to_hidden_workspace_sets_hidden_in_diff(mut s: StackSet<u8>) -> bool {
        let ss = s.snapshot();
        let tag = s.first_hidden_tag();

        let client = s.current_client().cloned();

        match (client, tag) {
            (Some(client), Some(tag)) => {
                s.move_client_to_tag(&client, &tag);
                let diff = Diff::from_raw(ss, &s, &[]);
                diff.hidden.contains(&client)
            }

            _ => true, // No hidden tags or no clients
        }
    }
}
