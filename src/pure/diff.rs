//! A diff of changes to pure State
use crate::pure::{geometry::Rect, screen::Screen};
use std::{collections::HashSet, hash::Hash, iter::once, mem::swap};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ScreenState<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub screen: usize,
    pub tag: String,
    pub clients: Vec<C>,
}

impl<C> From<&Screen<C>> for ScreenState<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    fn from(s: &Screen<C>) -> Self {
        Self {
            screen: s.index,
            tag: s.workspace.tag.clone(),
            clients: s.workspace.clients().copied().collect(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub focused_client: Option<C>,
    pub focused: ScreenState<C>,
    pub visible: Vec<ScreenState<C>>,
    pub positions: Vec<(C, Rect)>,
    pub hidden_clients: Vec<C>,
}

impl<C> Snapshot<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub(crate) fn visible_clients(&self) -> impl Iterator<Item = &C> {
        self.positions.iter().map(|(c, _)| c)
    }

    pub(crate) fn all_clients(&self) -> impl Iterator<Item = &C> {
        self.focused
            .clients
            .iter()
            .chain(self.visible.iter().flat_map(|s| s.clients.iter()))
            .chain(self.hidden_clients.iter())
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct Diff<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub before: Snapshot<C>,
    pub after: Snapshot<C>,
}

impl<C> Diff<C>
where
    C: Copy + Clone + PartialEq + Eq + Hash,
{
    pub fn new(before: Snapshot<C>, after: Snapshot<C>) -> Self {
        Self { before, after }
    }

    pub fn update(&mut self, after: Snapshot<C>) {
        swap(&mut self.before, &mut self.after);
        self.after = after;
    }

    pub fn focused_client(&self) -> Option<&C> {
        self.after.focused_client.as_ref()
    }

    pub fn focused_client_changed(&self) -> bool {
        self.before.focused_client != self.after.focused_client
    }

    pub fn client_changed_position(&self, id: &C) -> bool {
        let mut it = self.before.positions.iter();
        let before = it.find(|&(c, _)| c == id).map(|(_, r)| *r);
        let mut it = self.after.positions.iter();
        let after = it.find(|&(c, _)| c == id).map(|(_, r)| *r);

        before != after
    }

    pub fn newly_focused_screen(&self) -> Option<usize> {
        if self.before.focused.screen != self.after.focused.screen {
            Some(self.after.focused.screen)
        } else {
            None
        }
    }

    pub fn new_clients(&self) -> impl Iterator<Item = &C> {
        let before: HashSet<_> = self.before.all_clients().collect();

        self.after
            .all_clients()
            .filter(move |c| !before.contains(c))
    }

    pub fn hidden_clients(&self) -> impl Iterator<Item = &C> {
        let after: HashSet<_> = self.after.visible_clients().collect();

        self.before
            .visible_clients()
            .filter(move |c| !after.contains(c))
    }

    pub fn visible_clients(&self) -> impl Iterator<Item = &C> {
        self.after.visible_clients()
    }

    pub fn withdrawn_clients(&self) -> impl Iterator<Item = &C> {
        let after: HashSet<_> = self.after.all_clients().collect();

        self.before
            .all_clients()
            .filter(move |c| !after.contains(c))
    }

    pub fn previous_visible_tags(&self) -> HashSet<&str> {
        once(self.before.focused.tag.as_ref())
            .chain(self.before.visible.iter().map(|s| s.tag.as_ref()))
            .collect()
    }

    #[cfg(test)]
    pub fn current_visible_tags(&self) -> HashSet<&str> {
        once(self.after.focused.tag.as_ref())
            .chain(self.after.visible.iter().map(|s| s.tag.as_ref()))
            .collect()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        !(self.focused_client_changed()
            || self.newly_focused_screen().is_some()
            || self.new_clients().count() > 0
            || self.withdrawn_clients().count() > 0
            || self.previous_visible_tags() != self.current_visible_tags()
            || self.before.positions != self.after.positions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure::stack_set::tests::test_stack_set;
    use simple_test_case::test_case;

    #[test]
    fn diff_of_unchanged_stackset_is_empty() {
        let s = test_stack_set(5, 2);
        let positions: Vec<_> = s.iter_clients().map(|&c| (c, Rect::default())).collect();
        let ss = s.snapshot(positions);

        let diff = Diff::new(ss.clone(), ss);

        assert!(diff.is_empty())
    }

    #[test_case(Rect::new(0, 0, 10, 20), false; "unchanged")]
    #[test_case(Rect::new(0, 0, 20, 30), true; "changed")]
    #[test]
    fn client_changed_position_works(r: Rect, expected: bool) {
        let mut s = test_stack_set(1, 1);
        s.insert(1);
        let before = s.snapshot(vec![(1, Rect::new(0, 0, 10, 20))]);
        let after = s.snapshot(vec![(1, r)]);

        let diff = Diff::new(before, after);

        assert_eq!(diff.client_changed_position(&1), expected)
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use crate::{pure::StackSet, Xid};
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn diff_of_unchanged_stackset_is_empty(mut s: StackSet<Xid>) -> bool {
        let positions = s.visible_client_positions();
        let ss = s.snapshot(positions);
        let diff = Diff::new(ss.clone(), ss);

        diff.is_empty()
    }

    #[quickcheck]
    fn adding_a_client_is_new_in_diff(mut s: StackSet<Xid>) -> bool {
        let positions = s.visible_client_positions();
        let ss = s.snapshot(positions);
        let new = s.minimal_unknown_client();

        s.insert(new);

        let positions = s.visible_client_positions();
        let diff = Diff::new(ss, s.snapshot(positions));
        let res = diff.new_clients().any(|&c| c == new);

        res
    }

    // NOTE: Not checking that clients on the new workspace are visible as this is driven entirely by
    //       the positions returned by the Layout. In these tests, those are being specified manually
    //       so there is nothing to test.
    #[quickcheck]
    fn focusing_new_workspace_hides_old_clients_and_tag_in_diff(mut s: StackSet<Xid>) -> bool {
        let tag = match s.first_hidden_tag() {
            Some(t) => t,
            None => return true,
        };
        let prev_tag = s.current_tag().to_string();
        let clients_on_active: Vec<Xid> = match s.current_stack() {
            Some(stack) => stack.iter().cloned().collect(),
            None => vec![],
        };

        let positions = s.visible_client_positions();
        let ss = s.snapshot(positions);

        s.focus_tag(&tag);

        let positions = s.visible_client_positions();
        let diff = Diff::new(ss, s.snapshot(positions));
        let hidden: HashSet<_> = diff.hidden_clients().collect();

        let focused_clients_now_hidden = clients_on_active.iter().all(|c| hidden.contains(c));
        let tag_now_hidden = diff.previous_visible_tags().contains(&prev_tag.as_ref());

        focused_clients_now_hidden && tag_now_hidden
    }

    #[quickcheck]
    fn killing_focused_client_sets_withdrawn_and_hidden_in_diff(mut s: StackSet<Xid>) -> bool {
        let focus = match s.current_client() {
            Some(&c) => c,
            None => return true, // nothing to remove
        };

        let positions = s.visible_client_positions();
        let ss = s.snapshot(positions);
        s.remove_client(&focus);

        let positions = s.visible_client_positions();
        let diff = Diff::new(ss, s.snapshot(positions));
        let res = diff.withdrawn_clients().any(|&c| c == focus)
            && diff.hidden_clients().any(|&c| c == focus);

        res
    }

    #[quickcheck]
    fn moving_client_to_hidden_workspace_sets_hidden_in_diff(mut s: StackSet<Xid>) -> bool {
        let tag = s.first_hidden_tag();
        let client = s.current_client().cloned();

        match (client, tag) {
            (Some(client), Some(tag)) => {
                let positions = s.visible_client_positions();
                let ss = s.snapshot(positions);

                s.move_client_to_tag(&client, &tag);

                let positions = s.visible_client_positions();
                let diff = Diff::new(ss, s.snapshot(positions));
                let res = diff.hidden_clients().any(|&c| c == client);

                res
            }

            _ => true, // No hidden tags or no clients
        }
    }
}
