use crate::{
    core::layout::LayoutStack,
    pop_where,
    pure::{
        diff::{ScreenState, Snapshot},
        geometry::{Rect, RelativeRect, RelativeTo},
        workspace::check_workspace_invariants,
        Position, Screen, Stack, Workspace,
    },
    stack, Error, Result, Xid,
};
use std::{
    cmp::Ordering,
    collections::{HashMap, VecDeque},
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
    pub(crate) hidden: VecDeque<Workspace<C>>, // Workspaces not currently on any screen
    pub(crate) floating: HashMap<C, RelativeRect>, // Floating windows
    pub(crate) previous_tag: String,      // The last tag to be focused before the current one
    pub(crate) invisible_tags: Vec<String>, // Tags that should never be focused
    pub(crate) killed_clients: Vec<C>, // clients that have been removed and need processing on the X side
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
        floating: HashMap<C, RelativeRect>,
    ) -> Result<Self> {
        check_workspace_invariants(&workspaces)?;

        match (workspaces.len(), screen_details.len()) {
            (_, 0) => return Err(Error::NoScreens),
            (n_ws, n_screens) if n_ws < n_screens => {
                return Err(Error::InsufficientWorkspaces { n_ws, n_screens })
            }
            _ => (),
        }

        let hidden: VecDeque<Workspace<C>> = workspaces
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
            killed_clients: vec![],
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

    fn update_previous_tag(&mut self, new: String) {
        if self.invisible_tags.contains(&new) {
            return;
        }
        self.previous_tag = new;
    }

    /// Set focus to the [Workspace] with the specified tag.
    ///
    /// If there is no matching workspace then the [StackSet] is unmodified.
    /// If the [Workspace] is currently visible then focus moves to the screen
    /// containing that workspace, otherwise the workspace replaces whatever
    /// was on the active screen.
    ///
    /// If you always want to focus the given tag on the active screen, see
    /// [StackSet::pull_tag_to_screen] instead.
    pub fn focus_tag(&mut self, tag: impl AsRef<str>) {
        let tag = tag.as_ref();

        if self.screens.focus.workspace.tag == tag {
            return; // already focused
        }

        // If the tag is visible on another screen, focus moves to that screen
        if !self.try_cycle_screen_to_tag(tag) {
            // If the tag is hidden then it gets moved to the current screen
            self.try_swap_on_screen_workspace_with_hidden(tag);
        }

        // If nothing matched by this point then the requested tag is unknown
        // so there is nothing for us to do
    }

    fn try_cycle_screen_to_tag(&mut self, tag: &str) -> bool {
        let current_tag = self.screens.focus.workspace.tag.clone();

        loop {
            self.screens.focus_down();
            match &self.screens.focus.workspace.tag {
                // we've found and focused the tag
                t if t == tag => {
                    self.update_previous_tag(current_tag);
                    return true;
                }

                // we've looped so this tag isn't visible
                t if t == &current_tag => return false,

                // try the next tag
                _ => (),
            }
        }
    }

    fn try_swap_on_screen_workspace_with_hidden(&mut self, tag: &str) {
        if let Some(mut w) = pop_where!(self, hidden, |w: &Workspace<C>| w.tag == tag) {
            self.update_previous_tag(self.screens.focus.workspace.tag.clone());
            swap(&mut w, &mut self.screens.focus.workspace);
            self.hidden.push_back(w);
        }
    }

    // true if we swapped otherwise false
    fn try_swap_focused_workspace_with_tag(&mut self, tag: &str) -> bool {
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

    /// Focus the requested tag on the current screen, swapping the current
    /// tag with it.
    pub fn pull_tag_to_screen(&mut self, tag: impl AsRef<str>) {
        let tag = tag.as_ref();

        if self.screens.focus.workspace.tag == tag {
            return;
        }

        if !self.try_swap_focused_workspace_with_tag(tag) {
            self.try_swap_on_screen_workspace_with_hidden(tag);
        }
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

    pub(crate) fn float_unchecked<R: RelativeTo>(&mut self, client: C, r: R) {
        let screen = self.screen_for_client(&client).expect("client to be known");
        let r = r.relative_to(&screen.r);
        self.floating.insert(client, r);
    }

    /// Clear the floating status of a client, returning its previous preferred
    /// screen position if the client was known, otherwise `None`.
    pub fn sink(&mut self, client: &C) -> Option<Rect> {
        self.floating
            .remove(client)
            .map(|rr| rr.applied_to(&self.screens.focus.r))
    }

    /// Check whether a given tag currently has any floating windows present.
    ///
    /// Returns false if the tag given is unknown to this StackSet.
    pub fn has_floating_windows(&self, tag: impl AsRef<str>) -> bool {
        self.workspace(tag.as_ref())
            .map(|w| w.clients().any(|id| self.floating.contains_key(id)))
            .unwrap_or(false)
    }

    /// Delete a client from this [StackSet].
    pub fn remove_client(&mut self, client: &C) -> Option<C> {
        self.sink(client); // Clear any floating information we might have

        self.workspaces_mut()
            .map(|w| w.remove(client))
            .find(|opt| opt.is_some())
            .flatten()
    }

    /// Remove the currently focused client from this stack if there is one.
    ///
    /// The client is returned to the caller as `Some(C)` if there was one.
    pub fn remove_focused(&mut self) -> Option<C> {
        let client = self.current_client()?.clone();
        self.remove_client(&client)
    }

    /// Delete the currently focused client from this stack if there is one.
    ///
    /// The following diff will send a kill client message to this client on
    /// refresh.
    pub fn kill_focused(&mut self) {
        if let Some(client) = self.remove_focused() {
            self.killed_clients.push(client);
        }
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
            .workspaces_mut()
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

    /// Insert a client as the current focus for the given tag.
    ///
    /// NOTE: This will silently fail if the tag is not in the StackSet which
    ///       is why the method is not in the public API
    pub(crate) fn insert_as_focus_for(&mut self, tag: &str, c: C) {
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

    /// Is the given tag present in the [StackSet]?
    pub fn contains_tag(&self, tag: &str) -> bool {
        self.workspaces().any(|w| w.tag == tag)
    }

    /// All [Workspace] tags in this [StackSet] order by their id that have not been
    /// marked as being invisible.
    pub fn ordered_tags(&self) -> Vec<String> {
        let mut indexed: Vec<_> = self
            .workspaces()
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
            .workspaces()
            .filter(|w| !self.invisible_tags.contains(&w.tag))
            .collect();

        wss.sort_by_key(|w| w.id());

        wss.into_iter()
    }

    /// Find the tag of the [Workspace] currently displayed on [Screen] `index`.
    ///
    /// Returns [None] if the index is out of bounds
    pub fn tag_for_screen(&self, index: usize) -> Option<&str> {
        self.screens()
            .find(|s| s.index == index)
            .map(|s| s.workspace.tag.as_str())
    }

    /// Find the tag of the [Workspace] containing a given client.
    /// Returns Some(tag) if the client is known otherwise None.
    pub fn tag_for_client(&self, client: &C) -> Option<&str> {
        self.workspaces()
            .find(|w| {
                w.stack
                    .as_ref()
                    .map(|s| s.iter().any(|elem| elem == client))
                    .unwrap_or(false)
            })
            .map(|w| w.tag.as_str())
    }

    /// If the given client is currently visible on a screen return a
    /// reference to that screen, otherwise None.
    pub fn screen_for_client(&self, client: &C) -> Option<&Screen<C>> {
        self.screens.iter().find(|s| s.workspace.contains(client))
    }

    /// Find the tag of the [Workspace] with the given NetWmDesktop ID.
    pub fn tag_for_workspace_id(&self, id: usize) -> Option<String> {
        self.workspaces()
            .find(|w| w.id == id)
            .map(|w| w.tag.clone())
    }

    /// Returns `true` if the [StackSet] contains an element equal to the given value.
    pub fn contains(&self, client: &C) -> bool {
        self.clients().any(|c| c == client)
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

    /// An immutable reference to the currently focused [Screen]
    pub fn current_screen(&self) -> &Screen<C> {
        &self.screens.focus
    }

    /// An immutable reference to the current [Workspace]
    pub fn current_workspace(&self) -> &Workspace<C> {
        &self.screens.focus.workspace
    }

    /// A mutable reference to the current [Workspace]
    pub fn current_workspace_mut(&mut self) -> &mut Workspace<C> {
        &mut self.screens.focus.workspace
    }

    /// An immutable reference to the current [Stack] if there is one
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
            .workspaces()
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
        self.workspaces().find(|w| w.tag == tag)
    }

    /// A mutable reference to the [Workspace] with a tag of `tag` if there is one
    pub fn workspace_mut(&mut self, tag: &str) -> Option<&mut Workspace<C>> {
        self.workspaces_mut().find(|w| w.tag == tag)
    }

    /// Switch to the next available [Layout][crate::core::layout::Layout] on the focused [Workspace]
    pub fn next_layout(&mut self) {
        self.screens.focus.workspace.next_layout()
    }

    /// Switch to the previous available [Layout][crate::core::layout::Layout] on the focused [Workspace]
    pub fn previous_layout(&mut self) {
        self.screens.focus.workspace.previous_layout()
    }

    /// Move focus to the next [Screen]
    pub fn next_screen(&mut self) {
        if self.screens.len() == 1 {
            return;
        }

        self.update_previous_tag(self.screens.focus.workspace.tag.clone());
        self.screens.focus_down();
    }

    /// Move focus to the previous [Screen]
    pub fn previous_screen(&mut self) {
        if self.screens.len() == 1 {
            return;
        }

        self.update_previous_tag(self.screens.focus.workspace.tag.clone());
        self.screens.focus_up();
    }

    /// Drag the focused workspace onto the next [Screen], holding focus
    pub fn drag_workspace_forward(&mut self) {
        if self.screens.len() == 1 {
            return;
        }

        // We stash the previous tag so that we can restore it after we've
        // cycled the screens and pulled over tag we were on before.
        let true_previous_tag = self.previous_tag.clone();
        self.next_screen();
        self.try_swap_focused_workspace_with_tag(&self.previous_tag.clone());
        self.previous_tag = true_previous_tag;
    }

    /// Drag the focused workspace onto the previous [Screen], holding focus
    pub fn drag_workspace_backward(&mut self) {
        if self.screens.len() == 1 {
            return;
        }

        // We stash the previous tag so that we can restore it after we've
        // cycled the screens and pulled over tag we were on before.
        let true_previous_tag = self.previous_tag.clone();
        self.previous_screen();
        self.try_swap_focused_workspace_with_tag(&self.previous_tag.clone());
        self.previous_tag = true_previous_tag;
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
        self.workspaces_mut().find(|w| w.tag == tag).map(f);
    }

    /// Iterate over each [Screen] in this [StackSet] in an arbitrary order.
    pub fn screens(&self) -> impl Iterator<Item = &Screen<C>> {
        self.screens.iter()
    }

    /// Mutably iterate over each [Screen] in this [StackSet] in an arbitrary order.
    pub fn screens_mut(&mut self) -> impl Iterator<Item = &mut Screen<C>> {
        self.screens.iter_mut()
    }

    /// Iterate over each [Workspace] in this [StackSet] in an arbitrary order.
    pub fn workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.screens
            .iter()
            .map(|s| &s.workspace)
            .chain(self.hidden.iter())
    }

    /// Iterate over each non-hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn non_hidden_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.workspaces()
            .filter(|w| !self.invisible_tags.contains(&w.tag))
    }

    /// Mutably iterate over each [Workspace] in this [StackSet] in an arbitrary order.
    pub fn workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        self.screens
            .iter_mut()
            .map(|s| &mut s.workspace)
            .chain(self.hidden.iter_mut())
    }

    /// Iterate over the [Workspace] currently displayed on a screen in an arbitrary order.
    pub fn on_screen_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.screens.iter().map(|s| &s.workspace)
    }

    /// Iterate over the currently hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn hidden_workspaces(&self) -> impl Iterator<Item = &Workspace<C>> {
        self.hidden.iter()
    }

    /// Iterate over the currently hidden [Workspace] in this [StackSet] in an arbitrary order.
    pub fn hidden_workspaces_mut(&mut self) -> impl Iterator<Item = &mut Workspace<C>> {
        self.hidden.iter_mut()
    }

    /// Iterate over each client in this [StackSet] in an arbitrary order.
    pub fn clients(&self) -> impl Iterator<Item = &C> {
        self.workspaces().flat_map(|w| w.clients())
    }

    /// Iterate over clients present in on-screen Workspaces.
    ///
    /// *NOTE*: this does _not_ mean that every client returned by this iterator
    /// is visible on the screen: only that it is currently assigned to a workspace
    /// that is displayed on a screen.
    pub fn on_screen_workspace_clients(&self) -> impl Iterator<Item = &C> {
        self.on_screen_workspaces().flat_map(|w| w.clients())
    }

    /// Iterate over clients from workspaces not currently mapped to a screen.
    pub fn hidden_workspace_clients(&self) -> impl Iterator<Item = &C> {
        self.hidden_workspaces().flat_map(|w| w.clients())
    }
}

#[cfg(test)]
impl StackSet<Xid> {
    /// This is a test implementation that runs the `State::visible_client_positions`
    /// logic using a stub XConn and no layout hook.
    pub(crate) fn visible_client_positions(&self) -> Vec<(Xid, Rect)> {
        let mut s = crate::core::State {
            client_set: self.clone(),
            config: Default::default(),
            extensions: anymap::AnyMap::new(),
            root: Xid(0),
            mapped: Default::default(),
            pending_unmap: Default::default(),
            current_event: None,
            diff: Default::default(),
        };

        s.visible_client_positions(&crate::x::StubXConn)
    }

    /// This is a test implementation that runs the `State::position_and_snapshot`
    /// logic using a stub XConn and no layout hook.
    pub(crate) fn position_and_snapshot(&mut self) -> Snapshot<Xid> {
        let positions = self.visible_client_positions();
        self.snapshot(positions)
    }
}

impl StackSet<Xid> {
    /// Record a known client as floating, giving its preferred screen position.
    ///
    /// # Errors
    /// This method with return [Error::UnknownClient] if the given client is
    /// not already managed in this stack_set.
    ///
    /// This method with return [Error::ClientIsNotVisible] if the given client is
    /// not currently mapped to a screen. This is required to determine the correct
    /// relative positioning for the floating client as is it is moved between
    /// screens.
    pub fn float(&mut self, client: Xid, r: Rect) -> Result<()> {
        if !self.contains(&client) {
            return Err(Error::UnknownClient(client));
        }
        if self.screen_for_client(&client).is_none() {
            return Err(Error::ClientIsNotVisible(client));
        }

        self.float_unchecked(client, r);

        Ok(())
    }

    pub(crate) fn update_screens(&mut self, rects: Vec<Rect>) -> Result<()> {
        let n_old = self.screens.len();
        let n_new = rects.len();

        if n_new == 0 {
            return Err(Error::NoScreens);
        }

        match n_new.cmp(&n_old) {
            // Just a change in dimensions
            Ordering::Equal => (),

            // We have more screens now: pull in hidden workspaces to fill them
            // If we run out of workspaces we backfill using generated defaults
            Ordering::Greater => {
                let padding = self.take_from_hidden(n_new - n_old);
                for (n, w) in padding.into_iter().enumerate() {
                    self.screens.insert_at(
                        Position::Tail,
                        Screen {
                            workspace: w,
                            index: n_old + n,
                            r: Rect::default(),
                        },
                    );
                }
            }

            // We have fewer screens now: focus moves to the first screen and
            // we drop from the back of the stack
            Ordering::Less => {
                let mut raw = take(&mut self.screens).flatten();
                let removed = raw.split_off(n_new);
                self.hidden.extend(removed.into_iter().map(|s| s.workspace));
                self.screens = Stack::from_iter_unchecked(raw);
            }
        }

        // self.screens.len() is now correct so update the screen dimensions
        for (s, r) in self.screens.iter_mut().zip(rects) {
            s.r = r;
        }

        Ok(())
    }

    // This is a little fiddly...
    // Rather than hard erroring if we end up with new screens being detected that
    // push us over the number of available workspaces, we pad the workspace set
    // with ones we generate with default values. In doing this we need to make sure
    // that any _invisible_ workspaces are kept to one side so that they do not end
    // up focused on a screen by mistake.
    fn take_from_hidden(&mut self, n: usize) -> Vec<Workspace<Xid>> {
        let next_id = self.workspaces().map(|w| w.id).max().unwrap_or(0) + 1;
        let mut tmp = Vec::with_capacity(self.hidden.len());
        let mut hidden = VecDeque::new();

        // Filter out any hidden tags first
        for w in take(&mut self.hidden) {
            if self.invisible_tags.contains(&w.tag) {
                hidden.push_front(w);
            } else {
                tmp.push(w);
            }
        }

        // Sort so that we populate the new screens with workspaces in order of ID.
        // Without this we are basing things off of the order we ended up with after
        // whatever workspace focus changes the user has made while we are running.
        tmp.sort_by_key(|w| w.id);

        // Pad the remaining workspace count with empty default workspaces if we are
        // below the number we need.
        if tmp.len() < n {
            for m in 0..(n - tmp.len()) {
                tmp.push(Workspace::new_default(next_id + m));
            }
        } else {
            let extra = tmp.split_off(n);
            hidden.extend(extra);
        }

        self.hidden = hidden;

        tmp
    }
}

impl<C> StackSet<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) fn snapshot(&mut self, positions: Vec<(C, Rect)>) -> Snapshot<C> {
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
            hidden_clients: self.hidden_workspace_clients().copied().collect(),
            killed_clients: take(&mut self.killed_clients),
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
        let screens: Vec<Rect> = (0..(n_screens as u32))
            .map(|k| Rect::new(k * 1000, k * 2000, 1000, 2000))
            .collect();

        StackSet::try_new(LayoutStack::default(), tags, screens).unwrap()
    }

    pub fn test_stack_set(n_tags: usize, n_screens: usize) -> StackSet<u8> {
        _test_stack_set(n_tags, n_screens)
    }

    pub fn test_xid_stack_set(n_tags: usize, n_screens: usize) -> StackSet<Xid> {
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

        let visible_tags: Vec<&str> = s.screens().map(|s| s.workspace.tag.as_ref()).collect();

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
        let mut screen_indices: Vec<usize> = s.screens().map(|s| s.index).collect();
        screen_indices.sort();

        assert_eq!(screen_indices, vec![0, 1, 2])
    }

    #[test]
    fn iter_screens_mut_returns_all_screens() {
        let mut s = test_iter_stack_set();
        let mut screen_indices: Vec<usize> = s.screens_mut().map(|s| s.index).collect();
        screen_indices.sort();

        assert_eq!(screen_indices, vec![0, 1, 2])
    }

    #[test]
    fn iter_workspaces_returns_all_workspaces() {
        let s = test_iter_stack_set();
        let mut tags: Vec<&str> = s.workspaces().map(|w| w.tag.as_str()).collect();
        tags.sort();

        assert_eq!(tags, vec!["1", "2", "3", "4", "5"])
    }

    #[test]
    fn iter_workspaces_mut_returns_all_workspaces() {
        let mut s = test_iter_stack_set();
        let mut tags: Vec<&str> = s.workspaces_mut().map(|w| w.tag.as_str()).collect();
        tags.sort();

        assert_eq!(tags, vec!["1", "2", "3", "4", "5"])
    }

    #[test]
    fn iter_clients_returns_all_clients() {
        let s = test_iter_stack_set();
        let mut clients: Vec<u8> = s.clients().copied().collect();
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
            let mut cs: Vec<_> = s.clients().copied().collect();
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
    fn screen_change_focuses_new_screen(forward: bool, expected_index: usize) {
        let mut s = test_stack_set(5, 3);

        assert_eq!(s.current_screen().index(), 0);

        if forward {
            s.next_screen();
        } else {
            s.previous_screen();
        }

        assert_eq!(s.current_screen().index(), expected_index);
    }

    #[test_case(1, true, "1"; "single screen forward")]
    #[test_case(1, false, "1"; "single screen backward")]
    #[test_case(2, true, "3"; "two screens forward")]
    #[test_case(2, false, "3"; "two screens backward")]
    #[test]
    fn screen_change_sets_expected_previous_tag(n_screens: usize, forward: bool, tag: &str) {
        let mut s = test_stack_set(5, n_screens);

        s.focus_tag("3");

        assert_eq!(s.current_tag(), "3");
        assert_eq!(s.previous_tag, "1");

        if forward {
            s.next_screen();
        } else {
            s.previous_screen();
        }

        assert_eq!(s.previous_tag, tag);
    }

    #[test_case(true, 1; "forward")]
    #[test_case(false, 2; "backward")]
    #[test]
    fn drag_workspace_focuses_new_screen(forward: bool, expected_index: usize) {
        let mut s = test_stack_set(5, 3);

        assert_eq!(s.screens.focus.workspace.tag, "1");
        assert_eq!(s.screens.focus.index, 0);

        if forward {
            s.drag_workspace_forward();
        } else {
            s.drag_workspace_backward();
        }

        assert_eq!(s.screens.focus.workspace.tag, "1");
        assert_eq!(s.screens.focus.index, expected_index);
    }

    #[test_case(1, true; "single screen forward")]
    #[test_case(1, false; "single screen backward")]
    #[test_case(2, true; "two screens forward")]
    #[test_case(2, false; "two screens backward")]
    #[test]
    fn drag_workspace_maintains_previous_tag(n_screens: usize, forward: bool) {
        let mut s = test_stack_set(5, n_screens);
        s.focus_tag("3");

        // This state is technically invalid for us to get in to but the point is
        // to check that we definitely leave the previous tag alone during this
        // operation and don't end up with it anywhere it shouldn't be.
        s.previous_tag = "PREVIOUS".to_owned();

        assert_eq!(s.screens.focus.workspace.tag, "3");
        assert_eq!(s.previous_tag, "PREVIOUS");

        if forward {
            s.drag_workspace_forward();
        } else {
            s.drag_workspace_backward();
        }

        // We're keeping the same tag focused so we shouldn't have modified the
        // previous tag state at all regardless of screen count.
        assert_eq!(s.screens.focus.workspace.tag, "3");
        assert_eq!(s.previous_tag, "PREVIOUS");
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

    fn focused_tags(ss: &StackSet<Xid>) -> Vec<&String> {
        ss.screens.iter().map(|s| &s.workspace.tag).collect()
    }

    #[test_case(1, 1, 0, vec!["1"], vec!["1"]; "single to single")]
    #[test_case(1, 2, 0, vec!["1"], vec!["1", "2"]; "single to multiple no padding")]
    #[test_case(1, 3, 0, vec!["1"], vec!["1", "2", "WS-3"]; "single to multiple with padding")]
    #[test_case(2, 1, 0, vec!["1", "2"], vec!["1"]; "multiple to single")]
    #[test_case(2, 2, 1, vec!["1", "2"], vec!["1", "2"]; "multiple to same count")]
    #[test_case(2, 3, 1, vec!["1", "2"], vec!["1", "2", "WS-3"]; "multiple to more with padding")]
    #[test]
    fn update_screens(
        n_before: usize,
        n_after: usize,
        focus_after: usize,
        tags_before: Vec<&str>,
        tags_after: Vec<&str>,
    ) {
        let mut ss: StackSet<Xid> = StackSet::try_new(
            LayoutStack::default(),
            ["1", "2"],
            vec![Rect::default(); n_before],
        )
        .expect("enough workspaces to cover the number of initial screens");

        // Invisible workspaces should never have focus: backfilling from the currently
        // hidden workspaces needs to not put this on a screen.
        ss.add_invisible_workspace("INVISIBLE")
            .expect("no tag collisions");

        // Focus the last screen so that if we truncate we should fall back to
        // the first screen now that we are out of bounds
        ss.focus_screen(n_before - 1);

        assert_eq!(ss.screens.len(), n_before);
        assert_eq!(focused_tags(&ss), tags_before);

        ss.update_screens(vec![Rect::default(); n_after]).unwrap();

        assert_eq!(ss.screens.len(), n_after);
        assert_eq!(ss.screens.focus.index, focus_after);
        assert_eq!(focused_tags(&ss), tags_after);

        // Shouldn't have dropped any workspaces, only padded if needed.
        // The +1 here is for the invisible workspace
        let expected = std::cmp::max(2, n_after) + 1;
        assert_eq!(ss.workspaces().count(), expected);
    }

    #[test]
    fn update_screens_with_empty_vec_is_an_error() {
        let mut ss: StackSet<Xid> =
            StackSet::try_new(LayoutStack::default(), ["1", "2"], vec![Rect::default(); 2])
                .expect("enough workspaces to cover the number of screens");

        let res = ss.update_screens(vec![]);

        assert!(matches!(res, Err(Error::NoScreens)));
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
            self.workspaces()
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
        let target = match s.clients().max() {
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
