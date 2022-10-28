use crate::{
    core::layout::{IntoMessage, LayoutStack},
    pure::Stack,
    Error, Result,
};

#[derive(Debug, Clone)]
pub struct Workspace<C> {
    pub(crate) id: usize,
    pub(crate) tag: String,
    pub(crate) layouts: LayoutStack,
    pub(crate) stack: Option<Stack<C>>,
}

impl<C> Default for Workspace<C> {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tag: Default::default(),
            layouts: Default::default(),
            stack: Default::default(),
        }
    }
}

impl<C> Workspace<C> {
    pub fn new<T>(id: usize, tag: T, layouts: LayoutStack, stack: Option<Stack<C>>) -> Self
    where
        T: Into<String>,
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
            tag: id.to_string(),
            ..Self::default()
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn tag(&self) -> &str {
        &self.tag
    }

    pub fn layout_name(&self) -> String {
        self.layouts.focus.name()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.stack.is_none()
    }

    pub fn focus(&self) -> Option<&C> {
        self.stack.as_ref().map(|s| &s.focus)
    }

    pub fn clients(&self) -> impl Iterator<Item = &C> {
        self.stack.iter().flat_map(|s| s.iter())
    }

    pub fn clients_mut(&mut self) -> impl Iterator<Item = &mut C> {
        self.stack.iter_mut().flat_map(|s| s.iter_mut())
    }

    pub(crate) fn remove_focused(&mut self) -> Option<C> {
        let current = self.stack.take();
        let (focus, new_stack) = current?.remove_focused();
        self.stack = new_stack;

        Some(focus)
    }

    pub(crate) fn remove(&mut self, c: &C) -> Option<C>
    where
        C: PartialEq,
    {
        let current = self.stack.take();
        let (maybe_c, new_stack) = current?.remove(c);
        self.stack = new_stack;

        maybe_c
    }

    pub fn handle_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        self.layouts.handle_message(m)
    }

    pub fn broadcast_message<M>(&mut self, m: M)
    where
        M: IntoMessage,
    {
        self.layouts.broadcast_message(m)
    }

    pub fn next_layout(&mut self) {
        self.layouts.focus_down();
    }

    pub fn previous_layout(&mut self) {
        self.layouts.focus_up();
    }
}

impl<C: PartialEq> Workspace<C> {
    pub fn contains(&self, c: &C) -> bool {
        match &self.stack {
            Some(s) => s.contains(c),
            None => false,
        }
    }
}

pub(crate) fn check_workspace_invariants<C>(workspaces: &[Workspace<C>]) -> Result<()> {
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
    fn remove_returns_as_expected(stack: Option<Stack<u8>>, maybe_c: Option<u8>, is_some: bool) {
        let mut w = Workspace::new(0, "test", LayoutStack::default(), stack);

        assert_eq!(w.remove(&5), maybe_c);
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
