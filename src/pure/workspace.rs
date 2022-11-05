use crate::{
    core::layout::{IntoMessage, LayoutStack},
    pure::Stack,
    Error, Result,
};
use std::fmt;

/// A wrapper around a [Stack] of windows belonging to a single "workspace" or virtual
/// desktop. When this workspace is active on a given screen, the windows contained in
/// its stack will be positioned using the active layout of its [LayoutStack].
#[derive(Debug, Clone)]
pub struct Workspace<T> {
    pub(crate) id: usize,
    pub(crate) tag: String,
    pub(crate) layouts: LayoutStack,
    pub(crate) stack: Option<Stack<T>>,
}

impl<T> Default for Workspace<T> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tag: Default::default(),
            layouts: Default::default(),
            stack: Default::default(),
        }
    }
}

impl<T: fmt::Display> fmt::Display for Workspace<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let stack = self
            .stack
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_default();

        write!(
            f,
            "Workspace({}, {}):\n  - layouts: {}\n  - stack: {}",
            self.id, self.tag, self.layouts, stack
        )
    }
}

impl<T> Workspace<T> {
    /// Create a new Workspace with the given layouts and stack.
    pub fn new<S>(id: usize, tag: S, layouts: LayoutStack, stack: Option<Stack<T>>) -> Self
    where
        S: Into<String>,
    {
        Self {
            id,
            tag: tag.into(),
            layouts,
            stack,
        }
    }

    // Used for padding workspaces when needed in a rescreen event (see detect_screens in src/core/handle.rs)
    pub(crate) fn new_default(id: usize) -> Self {
        Self {
            id,
            tag: format!("WS-{id}"),
            ..Self::default()
        }
    }

    /// A fixed integer ID for this workspace.
    pub fn id(&self) -> usize {
        self.id
    }

    /// The string tag for this workspace.
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The name of the currently active layout being used by this workspace
    pub fn layout_name(&self) -> String {
        self.layouts.focus.name()
    }

    /// Whether or not this workspace currently holds any windows
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_none()
    }

    /// An immutable reference to the focused window for this workspace if there is one
    pub fn focus(&self) -> Option<&T> {
        self.stack.as_ref().map(|s| &s.focus)
    }

    /// An iterator over all windows in this workspace.
    pub fn clients(&self) -> impl Iterator<Item = &T> {
        self.stack.iter().flat_map(|s| s.iter())
    }

    pub(crate) fn remove_focused(&mut self) -> Option<T> {
        let current = self.stack.take();
        let (focus, new_stack) = current?.remove_focused();
        self.stack = new_stack;

        Some(focus)
    }

    /// Pass the given message on to the currently focused layout.
    pub fn handle_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        self.layouts.handle_message(m)
    }

    /// Pass the given message on to _all_ layouts available to this workspace.
    pub fn broadcast_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        self.layouts.broadcast_message(m)
    }

    /// Switch to the next available layout for this workspace.
    pub fn next_layout(&mut self) {
        self.layouts.focus_down();
    }

    /// Switch to the previous available layout for this workspace.
    pub fn previous_layout(&mut self) {
        self.layouts.focus_up();
    }
}

impl<T: PartialEq> Workspace<T> {
    /// Check if a given window is currently part of this workspace
    pub fn contains(&self, t: &T) -> bool {
        match &self.stack {
            Some(s) => s.contains(t),
            None => false,
        }
    }

    pub(crate) fn remove(&mut self, t: &T) -> Option<T> {
        let current = self.stack.take();
        let (maybe_t, new_stack) = current?.remove(t);
        self.stack = new_stack;

        maybe_t
    }
}

pub(crate) fn check_workspace_invariants<T>(workspaces: &[Workspace<T>]) -> Result<()> {
    let tags = workspaces.iter().map(|w| &w.tag);
    let mut seen = vec![];
    let mut duplicates = vec![];

    for tag in tags {
        if seen.contains(&tag) {
            duplicates.push(tag.to_owned());
        }
        seen.push(tag);
    }

    if !duplicates.is_empty() {
        duplicates.sort();
        duplicates.dedup();

        return Err(Error::NonUniqueTags { tags: duplicates });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack;
    use simple_test_case::test_case;

    #[test_case(Some(stack!([1, 2], 3, [4, 5])), Some(5), true; "known in stack")]
    #[test_case(Some(stack!(5)), Some(5), false; "known focus only")]
    #[test_case(Some(stack!([1, 2], 3, [4])), None, true; "unknown")]
    #[test_case(None, None, false; "empty stack")]
    #[test]
    fn remove_returns_as_expected(stack: Option<Stack<u8>>, maybe_t: Option<u8>, is_some: bool) {
        let mut w = Workspace::new(0, "test", LayoutStack::default(), stack);

        assert_eq!(w.remove(&5), maybe_t);
        assert_eq!(w.stack.is_some(), is_some);
    }

    #[test_case(&["1", "2", "3"], None; "no duplicate tags")]
    #[test_case(&["1", "2", "3", "2"], Some(&["2"]); "single duplicate")]
    #[test_case(&["1", "2", "3", "2", "3"], Some(&["2", "3"]); "multiple duplicates")]
    #[test_case(&["3", "2", "1", "2", "3"], Some(&["2", "3"]); "multiple duplicates sorted")]
    #[test]
    fn check_workspace_invariants(tags: &[&str], duplicates: Option<&[&str]>) {
        let workspaces: Vec<Workspace<u8>> = tags
            .iter()
            .enumerate()
            .map(|(i, tag)| Workspace::new(i, *tag, Default::default(), None))
            .collect();

        let res = check_workspace_invariants(&workspaces);

        match duplicates {
            None => assert!(res.is_ok()),
            Some(expected_tags) => match res {
                Err(Error::NonUniqueTags { tags }) => assert_eq!(tags, expected_tags),
                _ => panic!("expected NonUniqueTags, got {res:?}"),
            },
        }
    }
}
