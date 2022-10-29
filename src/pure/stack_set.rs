use crate::{
    core::layout::{Layout, LayoutStack},
    pop_where,
    pure::{
        diff::{ScreenState, Snapshot},
        geometry::Rect,
        workspace::check_workspace_invariants,
        Position, Screen, Stack, Workspace,
    },
    stack, Error, Result, Xid,
};
use std::{
    collections::{HashMap, LinkedList},
    hash::Hash,
    mem::{swap, take},
};

/// The side-effect free internal state representation of the window manager.
#[derive(Default, Debug, Clone)]
pub struct StackSet<C>
where
    C: Clone + PartialEq + Eq + Hash,
{
    pub(crate) screens: Stack<Screen<C>>, // Workspaces visible on screens
    pub(crate) hidden: LinkedList<Workspace<C>>, // Workspaces not currently on any screen
    pub(crate) floating: HashMap<C, Rect>, // Floating windows
    pub(crate) previous_tag: String,      // The last tag to be focused before the current one
    pub(crate) invisible_tags: Vec<String>, // Tags that should never be focused
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

        Self::try_new_concrete(workspaces, screen_details, HashMap::new())
    }

    pub(crate) fn try_new_concrete(
        mut workspaces: Vec<Workspace<C>>,
        screen_details: Vec<Rect>,
        floating: HashMap<C, Rect>,
    ) -> Result<Self> {
        check_workspace_invariants(&workspaces)?;

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

        let screens =
            Stack::from_iter_unchecked(workspaces.into_iter().zip(screen_details).enumerate().map(
                |(index, (workspace, r))| Screen {
                    workspace,
                    index,
                    r,
                },
            ));

        let previous_tag = screens.focus.workspace.tag.clone();

        Ok(Self {
            screens,
            hidden,
            floating,
            previous_tag,
            invisible_tags: vec![],
        })
    }

    /// Set focus to the [Screen] with the specified index.
    ///
    /// If there is no matching screen then the [StackSet] is unmodified.
    pub fn focus_screen(&mut self, screen_index: usize) {
        let current = self.screens.focus.index;
        if current == screen_index {
            return;
        }

        loop {
            self.screens.focus_down();
            if [current, screen_index].contains(&self.screens.focus.index) {
                break;
            }
        }
    }

    /// Set focus to the [Workspace] with the specified tag.
    ///
    /// If there is no matching workspace then the [StackSet] is unmodified.
    /// If the [Workspace] is currently visible it is swapped with the one
    /// on the active [Screen], otherwise the workspace replaces whatever
    /// was on the active screen.
    pub fn focus_tag(&mut self, tag: impl AsRef<str>) {
        let current_tag = self.screens.focus.workspace.tag.clone();
        let tag = tag.as_ref();

        if current_tag == tag {
            return; // already focused
        }

        // If the tag is visible on another screen, focus moves to that screen
        loop {
            self.screens.focus_down();
            match &self.screens.focus.workspace.tag {
                // we've found and focused the tag
                t if t == tag => {
                    self.previous_tag = current_tag;
                    return;
                }

                // we've looped so this tag isn't visible
                t if t == &current_tag => break,

                // try the next tag
                _ => (),
            }
        }

        // If the tag is hidden then it gets moved to the current screen
        if let Some(mut w) = pop_where!(self, hidden, |w: &Workspace<C>| w.tag == tag) {
            self.previous_tag = current_tag;
            swap(&mut w, &mut self.screens.focus.workspace);
            self.hidden.push_back(w);
        }

        // If nothing matched by this point then the requested tag is unknown
        // so there is nothing for us to do
    }

    /// Toggle focus back to the previously focused [Workspace] based on its tag
    pub fn toggle_tag(&mut self) {
        self.focus_tag(self.previous_tag.clone());
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

    /// Check whether a given tag currently has any floating windows present.
    ///
    /// Returns false if the tag given is unknown to this StackSet.
    pub fn has_floating_windows(&self, tag: impl AsRef<str>) -> bool {
        self.workspace(tag.as_ref())
            .map(|w| w.clients().any(|c| self.floating.contains_key(c)))
            .unwrap_or(false)
    }

    /// Delete a client from this [StackSet].
    pub fn remove_client(&mut self, client: &C) -> Option<C> {
        self.sink(client); // Clear any floating information we might have

        self.iter_workspaces_mut()
            .map(|w| w.remove(client))
            .find(|opt| opt.is_some())
            .flatten()
    }

    /// Delete the currently focused client from this stack if there is one.
    ///
    /// The client is returned to the caller as `Some(C)` if there was one.
    pub fn remove_focused(&mut self) -> Option<C> {
        self.screens.focus.workspace.remove_focused()
    }

    /// Delete the currently focused client from this stack if there is one.
    pub fn kill_focused(&mut self) {
        self.remove_focused();
    }

    /// Move the focused client of the current [Workspace] to the focused position
    /// of the workspace matching the provided `tag`.
    pub fn move_focused_to_tag(&mut self, tag: impl AsRef<str>) {
        let tag = tag.as_ref();
        if self.current_tag() == tag || !self.contains_tag(tag) {
            return;
        }

        let c = match self.screens.focus.workspace.remove_focused() {
            None => return,
            Some(c) => c,
        };

        self.insert_as_focus_for(tag, c)
    }

    /// Move the given client to the focused position of the [Workspace] matching
    /// the provided `tag`. If the client is already on the target workspace it is
    /// moved to the focused position.
    pub fn move_client_to_tag(&mut self, client: &C, tag: impl AsRef<str>) {
        let tag = tag.as_ref();

        if !self.contains_tag(tag) {
            return;
        }

        // Not calling self.remove_client as that will also sink the client if it
        // was floating
        let maybe_removed = self
            .iter_workspaces_mut()
            .map(|w| w.remove(client))
            .find(|opt| opt.is_some())
            .flatten();

        let c = match maybe_removed {
            None => return,
            Some(c) => c,
        };

        self.insert_as_focus_for(tag, c)
    }

    /// Move the given client to the focused position of the current [Workspace].
    /// If the client is already on the target workspace it is moved to the focused position.
    pub fn move_client_to_current_tag(&mut self, client: &C) {
        self.move_client_to_tag(client, self.screens.focus.workspace.tag.clone());
    }

    fn insert_as_focus_for(&mut self, tag: &str, c: C) {
        self.modify_workspace(tag, |w| {
            w.stack = Some(match take(&mut w.stack) {
                None => stack!(c),
                Some(mut s) => {
                    s.insert_at(Position::Focus, c);
                    s
                }
            });
        });
    }

    pub fn contains_tag(&self, tag: &str) -> bool {
        self.iter_workspaces().any(|w| w.tag == tag)
    }

    /// All [Workspace] tags in this [StackSet] order by their id that have not been
    /// marked as being invisible.
    pub fn ordered_tags(&self) -> Vec<String> {
        let mut indexed: Vec<_> = self
            .iter_workspaces()
            .map(|w| (w.id, w.tag.clone()))
            .filter(|(_, t)| !self.invisible_tags.contains(t))
            .collect();

        indexed.sort_by_key(|(id, _)| *id);

        indexed.into_iter().map(|(_, tag)| tag).collect()
    }

    /// All Workspaces in this [StackSet] order by their id that have not been
    /// marked as being invisible.
    pub fn ordered_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        let mut wss: Vec<_> = self
            .iter_workspaces()
            .filter(|w| !self.invisible_tags.contains(&w.tag))
            .collect();

        wss.sort_by_key(|w| w.id());

        wss.into_iter()
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

    /// Returns `true` if the [StackSet] contains a visible element equal to the given value.
    pub fn is_visible(&self, client: &C) -> bool {
        self.iter_visible_clients().any(|c| c == client)
    }

    /// Extract a reference to the focused element of the current [Stack]
    pub fn current_client(&self) -> Option<&C> {
        self.screens
            .focus
            .workspace
            .stack
            .as_ref()
            .map(|s| &s.focus)
    }

    pub fn current_screen(&self) -> &Screen<C> {
        &self.screens.focus
    }

    /// Get a reference to the current [Workspace]
    pub fn current_workspace(&self) -> &Workspace<C> {
        &self.screens.focus.workspace
    }

    /// Get a mutable reference to the current [Workspace]
    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        &mut self.screens.focus.workspace
    }

    /// Get a reference to the current [Stack] if there is one
    pub fn current_stack(&self) -> Option<&Stack<C>> {
        self.screens.focus.workspace.stack.as_ref()
    }

    /// The `tag` of the current [Workspace]
    pub fn current_tag(&self) -> &str {
        &self.screens.focus.workspace.tag
    }

    /// Add a new [Workspace] to this [StackSet].
    ///
    /// The id assigned to this workspace will be max(workspace ids) + 1.
    ///
    /// # Errors
    /// This function will error with `NonUniqueTags` if the given tag is already present.
    pub fn add_workspace<T>(&mut self, tag: T, layouts: LayoutStack) -> Result<()>
    where
        T: Into<String>,
    {
        let tag = tag.into();
        if self.contains_tag(&tag) {
            return Err(Error::NonUniqueTags { tags: vec![tag] });
        }

        let id = self
            .iter_workspaces()
            .map(|w| w.id)
            .max()
            .expect("at least one workspace")
            + 1;
        let ws = Workspace::new(id, tag, layouts, None);
        self.hidden.push_front(ws);

        Ok(())
    }

    /// Add a new invisible [Workspace] to this [StackSet].
    ///
    /// It will not be possible to focus this workspace on a screen but its
    /// state will be tracked and clients can be placed on it.
    /// The id assigned to this workspace will be max(workspace ids) + 1.
    ///
    /// # Errors
    /// This function will error with `NonUniqueTags` if the given tag is already present.
    pub fn add_invisible_workspace<T>(&mut self, tag: T) -> Result<()>
    where
        T: Into<String>,
    {
        let tag = tag.into();
        self.add_workspace(tag.clone(), LayoutStack::default())?;
        self.invisible_tags.push(tag);

        Ok(())
    }

    /// A reference to the [Workspace] with a tag of `tag` if there is one
    pub fn workspace(&self, tag: &str) -> Option<&Workspace<C>> {
        self.iter_workspaces().find(|w| w.tag == tag)
    }

    /// A mutable reference to the [Workspace] with a tag of `tag` if there is one
    pub fn workspace_mut(&mut self, tag: &str) -> Option<&mut Workspace<C>> {
        self.iter_workspaces_mut().find(|w| w.tag == tag)
    }

    /// Switch to the next available [Layout] on the focused [Workspace]
    pub fn next_layout(&mut self) {
        self.screens.focus.workspace.next_layout()
    }

    /// Switch to the previous available [Layout] on the focused [Workspace]
    pub fn previous_layout(&mut self) {
        self.screens.focus.workspace.previous_layout()
    }

    /// Move focus to the next [Screen]
    pub fn next_screen(&mut self) {
        self.screens.focus_down();
    }

    /// Move focus to the previous [Screen]
    pub fn previous_screen(&mut self) {
        self.screens.focus_up();
    }

    // true if we swapped otherwise false
    fn swap_focused_workspace_with_tag(&mut self, tag: &str) -> bool {
        if self.screens.focus.workspace.tag == tag {
            return false;
        }

        let p = |s: &&mut Screen<C>| s.workspace.tag == tag;

        let in_up = self.screens.up.iter_mut().find(p);
        let in_down = self.screens.down.iter_mut().find(p);

        if let Some(s) = in_up.or(in_down) {
            swap(&mut self.screens.focus.workspace, &mut s.workspace);
            return true;
        }

        false
    }

    pub fn drag_workspace_forward(&mut self) {
        self.next_screen();
        self.swap_focused_workspace_with_tag(&self.previous_tag.clone());
    }

    pub fn drag_workspace_backward(&mut self) {
        self.previous_screen();
        self.swap_focused_workspace_with_tag(&self.previous_tag.clone());
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
        self.screens.focus.workspace.stack = f(take(&mut self.screens.focus.workspace.stack));
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
        self.screens.iter()
    }

    /// Mutably iterate over each [Screen] in this [StackSet] in an arbitrary order.
    pub fn iter_screens_mut(&mut self) -> impl Iterator<Item = &mut Screen<C>> {
        self.screens.iter_mut()
    }

    /// Iterate over each [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.screens
            .iter()
            .map(|s| &s.workspace)
            .chain(self.hidden.iter())
    }

    /// Mutably iterate over each [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        self.screens
            .iter_mut()
            .map(|s| &mut s.workspace)
            .chain(self.hidden.iter_mut())
    }

    /// Iterate over the currently visible [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_visible_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.screens.iter().map(|s| &s.workspace)
    }

    /// Iterate over the currently hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_hidden_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.hidden.iter()
    }

    /// Iterate over the currently hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn iter_hidden_workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        self.hidden.iter_mut()
    }

    /// Iterate over each client in this [StackSet] in an arbitrary order.
    pub fn iter_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_workspaces()
            .flat_map(|w| w.stack.iter().flat_map(|s| s.iter()))
    }

    /// Iterate over the currently visible clients in this [StackSet] in an arbitrary order.
    pub fn iter_visible_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_visible_workspaces()
            .flat_map(|w| w.stack.iter().flat_map(|s| s.iter()))
    }

    /// Iterate over the currently hidden clients in this [StackSet] in an arbitrary order.
    pub fn iter_hidden_clients(&self) -> impl Iterator<Item = &C> {
        self.iter_hidden_workspaces()
            .flat_map(|w| w.stack.iter().flat_map(|s| s.iter()))
    }

    /// Iterate over each client in this [StackSet] in an arbitrary order.
    pub fn iter_clients_mut(&mut self) -> impl Iterator<Item = &mut C> {
        self.iter_workspaces_mut()
            .flat_map(|w| w.stack.iter_mut().flat_map(|s| s.iter_mut()))
    }
}

impl StackSet<Xid> {
    /// Run the per-workspace layouts to get a screen position for each visible client. Floating clients
    /// are placed above stacked clients, clients per workspace are stacked in the order they are returned
    /// from the layout.
    /// NOTE: we require Xid as the client type here as we need that when running layouts
    pub(crate) fn visible_client_positions(&mut self) -> Vec<(Xid, Rect)> {
        let mut float_positions: Vec<(Xid, Rect)> = self
            .iter_visible_clients()
            .flat_map(|c| self.floating.get(c).map(|r| (*c, *r)))
            .collect();

        float_positions.reverse();

        let mut positions: Vec<(Xid, Rect)> = Vec::new();

        for s in self.screens.iter_mut() {
            let r = s.geometry();
            let tag = &s.workspace.tag;
            let true_stack = s.workspace.stack.as_ref();
            let tiling =
                true_stack.and_then(|st| st.from_filtered(|c| self.floating.get(c).is_none()));

            // TODO: if this supports using X state for determining layout position in future then this
            //       will be fallible and needs to fall back to a default layout.
            let (_, stack_positions) = s.workspace.layouts.layout_workspace(tag, &tiling, r);

            positions.extend(stack_positions.into_iter().rev());
        }

        positions.extend(float_positions);

        positions
    }
}

impl<C> StackSet<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) fn snapshot(&self, positions: Vec<(C, Rect)>) -> Snapshot<C> {
        let visible = self
            .screens
            .unravel()
            .skip(1) // skip the focused element
            .map(ScreenState::from)
            .collect();

        Snapshot {
            focused_client: self.current_client().copied(),
            focused: ScreenState::from(&self.screens.focus),
            visible,
            positions,
            hidden_clients: self.iter_hidden_clients().copied().collect(),
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
                $(#[$doc_str])*
                pub fn $method(&mut self) {
                    if let Some(ref mut stack) = self.screens.focus.workspace.stack {
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
    rotate_down,
    /// Rotate the Stack until the current focused element is in the head position.
    /// This is a no-op if the current stack is empty.
    rotate_focus_to_head,
    /// Move focus to the element in the head position.
    /// This is a no-op if the current stack is empty.
    focus_head,
    /// Swap the current head element with the focused element in the
    /// stack order. Focus stays with the original focused element.
    /// This is a no-op if the current stack is empty.
    swap_focus_and_head
);

#[cfg(test)]
pub mod tests {
    use super::*;
    use simple_test_case::test_case;

    fn _test_stack_set<C>(n_tags: usize, n_screens: usize) -> StackSet<C>
    where
        C: Copy + Clone + PartialEq + Eq + Hash,
    {
        let tags = (1..=n_tags).map(|n| n.to_string());
        let screens = vec![Rect::new(0, 0, 2000, 1000); n_screens];

        StackSet::try_new(LayoutStack::default(), tags, screens).unwrap()
    }

    pub fn test_stack_set(n_tags: usize, n_screens: usize) -> StackSet<u8> {
        _test_stack_set(n_tags, n_screens)
    }

    fn test_xid_stack_set(n_tags: usize, n_screens: usize) -> StackSet<Xid> {
        _test_stack_set(n_tags, n_screens)
    }

    pub fn test_stack_set_with_stacks<C>(stacks: Vec<Option<Stack<C>>>, n: usize) -> StackSet<C>
    where
        C: Copy + Clone + PartialEq + Eq + Hash,
    {
        let workspaces: Vec<Workspace<C>> = stacks
            .into_iter()
            .enumerate()
            .map(|(i, s)| Workspace::new(i, (i + 1).to_string(), LayoutStack::default(), s))
            .collect();

        match StackSet::try_new_concrete(
            workspaces,
            (0..(n as u32))
                .map(|k| Rect::new(k * 1000, k * 2000, 1000, 2000))
                .collect(),
            HashMap::new(),
        ) {
            Ok(s) => s,
            Err(e) => panic!("{e}"),
        }
    }

    #[test_case("1", &["1", "2"]; "current focused workspace")]
    #[test_case("2", &["1", "2"]; "visible on other screen")]
    #[test_case("3", &["3", "2"]; "currently hidden")]
    #[test]
    fn focus_tag_sets_correct_visible_workspaces(target: &str, vis: &[&str]) {
        let mut s = test_stack_set(5, 2);

        s.focus_tag(target);

        let visible_tags: Vec<&str> = s.iter_screens().map(|s| s.workspace.tag.as_ref()).collect();

        assert_eq!(s.screens.focus.workspace.tag, target);
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
        let mut clients: Vec<u8> = s.iter_clients().copied().collect();
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

    #[test]
    fn changing_workspace_retains_clients() {
        let mut s = test_stack_set_with_stacks(vec![Some(stack!(1)), Some(stack!(2, 3)), None], 1);

        let clients = |s: &StackSet<u8>| {
            let mut cs: Vec<_> = s.iter_clients().copied().collect();
            cs.sort();

            cs
        };

        assert_eq!(clients(&s), vec![1, 2, 3]);
        s.focus_tag("2");
        assert_eq!(clients(&s), vec![1, 2, 3]);
    }

    #[test_case(true, 1; "forward")]
    #[test_case(false, 2; "backward")]
    #[test]
    fn drag_workspace_focuses_new_screen(forward: bool, expected_index: usize) {
        let mut s = test_stack_set(5, 3);

        assert_eq!(s.current_tag(), "1");
        assert_eq!(s.current_screen().index(), 0);

        if forward {
            s.drag_workspace_forward();
        } else {
            s.drag_workspace_backward();
        }

        assert_eq!(s.current_tag(), "1");
        assert_eq!(s.current_screen().index(), expected_index);
    }

    #[test]
    fn floating_layer_clients_hold_focus() {
        let mut s = test_stack_set(5, 3);

        for n in 1..5 {
            s.insert(n);
        }

        s.float_unchecked(4, Rect::default());

        assert_eq!(s.current_client(), Some(&4));
    }

    #[test_case(1, "1"; "current focus to current tag")]
    #[test_case(2, "1"; "from current tag to current tag")]
    #[test_case(6, "1"; "from other tag to current tag")]
    #[test_case(6, "2"; "from other tag to same tag")]
    #[test_case(0, "2"; "from current tag to other tag")]
    #[test_case(7, "3"; "from other tag to other tag")]
    #[test_case(7, "4"; "from other tag to empty tag")]
    #[test]
    fn move_client_to_tag(client: u8, tag: &str) {
        let mut s = test_stack_set_with_stacks(
            vec![
                Some(stack!([0], 1, [2, 3])),
                Some(stack!([6, 7], 8)),
                Some(stack!(4, [5])),
                None,
            ],
            1,
        );

        s.move_client_to_tag(&client, tag);

        assert_eq!(s.workspace(tag).unwrap().focus(), Some(&client));
    }

    mod visible_client_positions {
        use super::*;

        fn stack_order(s: &mut StackSet<Xid>) -> Vec<u32> {
            let positions = s.visible_client_positions();
            positions.iter().map(|&(id, _)| *id).collect()
        }

        #[test]
        fn floating_windows_are_returned_last() {
            let mut s = test_xid_stack_set(5, 2);

            for n in 1..6 {
                s.insert(Xid(n));
            }

            s.float_unchecked(Xid(2), Rect::new(0, 0, 42, 42));
            s.float_unchecked(Xid(3), Rect::new(0, 0, 69, 69));

            assert_eq!(stack_order(&mut s), vec![1, 4, 5, 2, 3]);
        }

        #[test]
        fn newly_added_windows_are_below_floating() {
            let mut s = test_xid_stack_set(5, 2);

            for n in 1..6 {
                s.insert(Xid(n));
            }

            s.float_unchecked(Xid(2), Rect::new(0, 0, 42, 42));
            s.float_unchecked(Xid(3), Rect::new(0, 0, 69, 69));

            s.insert(Xid(6));

            assert_eq!(stack_order(&mut s), vec![1, 4, 5, 6, 2, 3]);
        }

        #[test]
        fn floating_clients_dont_break_insert_focus() {
            let mut s = test_xid_stack_set(1, 1);

            s.insert_at(Position::Focus, Xid(0));
            s.float_unchecked(Xid(0), Rect::new(0, 0, 42, 42));

            assert_eq!(s.current_client(), Some(&Xid(0)));

            // Each time we add a client it should be the focus
            // and the floating window should be stacked above
            // all others.
            let mut expected = vec![0];
            for n in 1..=5 {
                s.insert_at(Position::Focus, Xid(n));
                assert_eq!(s.current_client(), Some(&Xid(n)));

                // Tiled position ordering is reversed in visible_client_positions
                // in order to ensure that when we restack, the order returned
                // is from bottom -> top of the stack to make `restack` simpler to
                // implement.
                expected.insert(expected.len() - 1, n);
                assert_eq!(stack_order(&mut s), expected, "{:?}", s.current_stack());
            }
        }
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::{tests::test_stack_set_with_stacks, *};
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::collections::HashSet;

    impl<C> Stack<C>
    where
        C: Copy + Clone + PartialEq + Eq + Hash,
    {
        pub fn try_from_arbitrary_vec(mut up: Vec<C>, g: &mut Gen) -> Option<Self> {
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

    impl StackSet<Xid> {
        pub fn minimal_unknown_client(&self) -> Xid {
            let mut c = 0;

            while self.contains(&Xid(c)) {
                c += 1;
            }

            Xid(c)
        }

        pub fn first_hidden_tag(&self) -> Option<String> {
            self.hidden.iter().map(|w| w.tag.clone()).next()
        }

        pub fn last_tag(&self) -> String {
            self.iter_workspaces()
                .last()
                .expect("at least one workspace")
                .tag
                .clone()
        }

        pub fn last_visible_client(&self) -> Option<&Xid> {
            self.screens
                .down
                .back()
                .unwrap_or(&self.screens.focus)
                .workspace
                .stack
                .iter()
                .flat_map(|s| s.iter())
                .last()
        }
    }

    impl Arbitrary for Xid {
        fn arbitrary(g: &mut Gen) -> Self {
            Xid(u32::arbitrary(g))
        }
    }

    // For the tests below we only care about the stack structure not the elements themselves, so
    // we use `u8` as an easily defaultable focus if `Vec::arbitrary` gives us an empty vec.
    impl Arbitrary for StackSet<Xid> {
        fn arbitrary(g: &mut Gen) -> Self {
            let n_stacks = usize::arbitrary(g) % 10;
            let mut stacks = Vec::with_capacity(n_stacks);

            let mut clients: Vec<Xid> = HashSet::<Xid>::arbitrary(g).into_iter().collect();

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
    fn insert_pushes_to_current_stack(mut s: StackSet<Xid>) -> bool {
        let new_focus = s.minimal_unknown_client();
        s.insert(new_focus);

        s.current_client() == Some(&new_focus)
    }

    #[quickcheck]
    fn focus_client_focused_the_enclosing_workspace(mut s: StackSet<Xid>) -> bool {
        let target = match s.iter_clients().max() {
            Some(target) => *target,
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
    fn move_focused_to_tag(mut s: StackSet<Xid>) -> bool {
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
    fn move_client_to_tag(mut s: StackSet<Xid>) -> bool {
        let tag = s.last_tag();

        let c = match s.last_visible_client() {
            Some(&c) => c,
            None => return true, // no client to move for this case
        };

        s.move_client_to_tag(&c, &tag);
        s.focus_tag(&tag);

        s.current_client() == Some(&c)
    }
}
