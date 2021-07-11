//! An internal data structure and associated helpers for simplifying actions around
//! manipulating focusable ordered collections.

use crate::core::xconnection::Xid;

use std::{
    collections::VecDeque,
    fmt,
    iter::{FromIterator, IntoIterator},
    ops::{Index, IndexMut},
};

/// A direction to permute a Ring
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Direction {
    /// increase the index, wrapping if needed
    Forward,
    /// decrease the index, wrapping if needed
    Backward,
}

impl Direction {
    /// Invert this Direction
    pub fn reverse(&self) -> Direction {
        match self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}

/// Where a given element should be inserted into a Ring
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum InsertPoint {
    /// At the specified index (last if out of bounds)
    Index(usize),
    /// In place of the current focused element (pushing focused and later down in the stack)
    Focused,
    /// After the current focused element (pushing later elements down in the stack)
    AfterFocused,
    /// As the first element in the stack
    First,
    /// As the last element in the stack
    Last,
}

/// Used with WindowManager helper functions to select an element from the
/// known workspaces or clients.
#[derive(Clone, Copy)]
pub enum Selector<'a, T> {
    /// Any element in the target collection.
    ///
    /// For functions returning a single elemt this is equivalent to `Focused`, for functions
    /// returning multiple elements this will return the entire collection.
    Any,
    /// The focused element of the target collection.
    Focused,
    /// The element at this index.
    Index(usize),
    /// The element with/containing this client ID.
    WinId(Xid),
    /// The first element satisfying this condition.
    Condition(&'a dyn Fn(&T) -> bool),
}

impl<'a, T> fmt::Debug for Selector<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.debug_struct("Selector::Any").finish(),
            Self::Focused => f.debug_struct("Selector::Focused").finish(),
            Self::Index(i) => f.debug_struct("Selector::Index").field("index", i).finish(),
            Self::WinId(i) => f.debug_struct("Selector::WinId").field("id", i).finish(),
            Self::Condition(_func) => f
                .debug_struct("Selector::Condition")
                .field("condition", &stringify!(_func))
                .finish(),
        }
    }
}

/**
 * A Collection<T> that has both an order for its elements and a focused element
 * at some index.
 *
 * Supports rotating the position of the elements and rotating which element
 * is focused independently of one another.
 */
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Ring<T> {
    elements: VecDeque<T>,
    focused: usize,
}

impl<T> Default for Ring<T> {
    fn default() -> Self {
        Self {
            elements: VecDeque::new(),
            focused: 0,
        }
    }
}

impl<T> Ring<T> {
    pub fn new(elements: Vec<T>) -> Ring<T> {
        Ring {
            elements: elements.into(),
            focused: 0,
        }
    }

    pub fn would_wrap(&self, dir: Direction) -> bool {
        let wrap_back = self.focused == 0 && dir == Direction::Backward;
        let wrap_forward = self.focused == self.elements.len() - 1 && dir == Direction::Forward;

        wrap_back || wrap_forward
    }

    pub fn focused_index(&self) -> usize {
        self.focused
    }

    pub fn focused(&self) -> Option<&T> {
        self.elements.get(self.focused)
    }

    pub fn focused_unchecked(&self) -> &T {
        &self.elements[self.focused]
    }

    pub fn focused_mut(&mut self) -> Option<&mut T> {
        self.elements.get_mut(self.focused)
    }

    pub fn focused_mut_unchecked(&mut self) -> &mut T {
        &mut self.elements[self.focused]
    }

    pub fn rotate(&mut self, direction: Direction) {
        if self.elements.is_empty() {
            return;
        }
        match direction {
            Direction::Forward => self.elements.rotate_right(1),
            Direction::Backward => self.elements.rotate_left(1),
        }
    }

    fn next_index(&self, direction: Direction) -> usize {
        let max = self.elements.len() - 1;
        match direction {
            Direction::Forward => {
                if self.focused == max {
                    0
                } else {
                    self.focused + 1
                }
            }
            Direction::Backward => {
                if self.focused == 0 {
                    max
                } else {
                    self.focused - 1
                }
            }
        }
    }

    pub fn cycle_focus(&mut self, direction: Direction) -> Option<&T> {
        self.focused = self.next_index(direction);
        self.focused()
    }

    pub fn drag_focused(&mut self, direction: Direction) -> Option<&T> {
        match (self.focused, self.next_index(direction), direction) {
            (0, _, Direction::Backward) => self.rotate(direction),
            (_, 0, Direction::Forward) => self.rotate(direction),
            (focused, other, _) => self.elements.swap(focused, other),
        }

        self.cycle_focus(direction)
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn insert_at(&mut self, insert_point: &InsertPoint, element: T) {
        match insert_point {
            InsertPoint::Index(ix) => self.elements.insert(*ix, element),
            InsertPoint::Focused => self.elements.insert(self.focused_index(), element),
            InsertPoint::First => self.elements.push_front(element),
            InsertPoint::Last => self.elements.push_back(element),
            InsertPoint::AfterFocused => {
                let ix = self.focused_index() + 1;
                if ix > self.elements.len() {
                    self.elements.push_back(element)
                } else {
                    self.elements.insert(ix, element)
                }
            }
        }
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.elements.insert(index, element);
    }

    pub fn push(&mut self, element: T) {
        self.elements.push_back(element);
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, T> {
        self.elements.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<'_, T> {
        self.elements.iter_mut()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.elements.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.elements.get_mut(index)
    }

    pub fn vec_map<F: FnMut(&T) -> U, U>(&self, f: F) -> Vec<U> {
        self.elements.iter().map(f).collect()
    }

    pub fn apply_to<F: FnMut(&mut T)>(&mut self, s: &Selector<'_, T>, mut f: F) {
        if let Some(index) = self.index(s) {
            f(&mut self.elements[index]);
        }
    }

    fn clamp_focus(&mut self) {
        if self.focused > 0 && self.focused >= self.elements.len() - 1 {
            self.focused -= 1;
        }
    }

    fn element_by(&self, cond: impl Fn(&T) -> bool) -> Option<(usize, &T)> {
        self.elements.iter().enumerate().find(|(_, e)| cond(*e))
    }

    fn element_by_mut(&mut self, cond: impl Fn(&T) -> bool) -> Option<(usize, &mut T)> {
        self.elements.iter_mut().enumerate().find(|(_, e)| cond(*e))
    }

    pub fn index(&self, s: &Selector<'_, T>) -> Option<usize> {
        match s {
            Selector::WinId(_) => None, // ignored
            Selector::Focused | Selector::Any => Some(self.focused_index()),
            Selector::Index(i) => {
                if *i < self.len() {
                    Some(*i)
                } else {
                    None
                }
            }
            Selector::Condition(f) => self.element_by(f).map(|(i, _)| i),
        }
    }

    pub fn indexed_element(&self, s: &Selector<'_, T>) -> Option<(usize, &T)> {
        self.index(s).map(|i| (i, &self.elements[i]))
    }

    pub fn element(&self, s: &Selector<'_, T>) -> Option<&T> {
        match s {
            Selector::Focused | Selector::Any => self.focused(),
            Selector::Index(i) => self.elements.get(*i),
            Selector::WinId(_) => None, // ignored
            Selector::Condition(f) => self.element_by(f).map(|(_, e)| e),
        }
    }

    pub fn element_mut(&mut self, s: &Selector<'_, T>) -> Option<&mut T> {
        match s {
            Selector::Focused | Selector::Any => self.focused_mut(),
            Selector::Index(i) => self.elements.get_mut(*i),
            Selector::WinId(_) => None, // ignored
            Selector::Condition(f) => self.element_by_mut(f).map(|(_, e)| e),
        }
    }

    pub fn all_elements(&self, s: &Selector<'_, T>) -> Vec<&T> {
        match s {
            Selector::Any => self.iter().collect(),
            Selector::Focused => self.focused().into_iter().collect(),
            Selector::Index(i) => self.elements.get(*i).into_iter().collect(),
            Selector::WinId(_) => vec![], // ignored
            Selector::Condition(f) => self.elements.iter().filter(|e| f(*e)).collect(),
        }
    }

    pub fn all_elements_mut(&mut self, s: &Selector<'_, T>) -> Vec<&mut T> {
        match s {
            Selector::Any => self.iter_mut().collect(),
            Selector::Focused => self.focused_mut().into_iter().collect(),
            Selector::Index(i) => self.elements.get_mut(*i).into_iter().collect(),
            Selector::WinId(_) => vec![], // ignored
            Selector::Condition(f) => self.elements.iter_mut().filter(|e| f(*e)).collect(),
        }
    }

    pub fn focus(&mut self, s: &Selector<'_, T>) -> Option<(bool, &T)> {
        if self.index(s) == Some(self.focused) {
            return Some((false, &self.elements[self.focused]));
        }

        match s {
            Selector::Focused | Selector::Any => self.focused().map(|t| (true, t)),
            Selector::Index(i) => {
                self.focused = *i;
                self.focused().map(|t| (true, t))
            }
            Selector::WinId(_) => None, // ignored
            Selector::Condition(f) => {
                if let Some((i, _)) = self.element_by(f) {
                    self.focused = i;
                    Some((true, &self.elements[self.focused]))
                } else {
                    None
                }
            }
        }
    }

    pub fn remove(&mut self, s: &Selector<'_, T>) -> Option<T> {
        match s {
            Selector::Focused | Selector::Any => {
                let c = self.elements.remove(self.focused);
                self.clamp_focus();
                c
            }
            Selector::Index(i) => {
                let c = self.elements.remove(*i);
                self.clamp_focus();
                c
            }
            Selector::WinId(_) => None, // ignored
            Selector::Condition(f) => {
                if let Some((i, _)) = self.element_by(f) {
                    let c = self.elements.remove(i);
                    self.clamp_focus();
                    c
                } else {
                    None
                }
            }
        }
    }
}

impl<T: PartialEq> Ring<T> {
    pub fn equivalent_selectors(&self, s: &Selector<'_, T>, t: &Selector<'_, T>) -> bool {
        match (self.element(&s), self.element(&t)) {
            (Some(e), Some(f)) => e == f,
            _ => false,
        }
    }
}

impl<T: Clone> Ring<T> {
    #[allow(dead_code)]
    pub fn as_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

impl<T> Index<usize> for Ring<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl<T> IndexMut<usize> for Ring<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.elements[index]
    }
}

impl<T> FromIterator<T> for Ring<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut ring = Ring::new(Vec::new());
        for element in iter {
            ring.push(element);
        }

        ring
    }
}

impl<T> IntoIterator for Ring<T> {
    type Item = T;
    type IntoIter = std::collections::vec_deque::IntoIter<T>;

    /// Consumes the `VecDeque` into a front-to-back iterator yielding elements by
    /// value.
    fn into_iter(self) -> std::collections::vec_deque::IntoIter<T> {
        self.elements.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a Ring<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;

    fn into_iter(self) -> std::collections::vec_deque::Iter<'a, T> {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Ring<T> {
    type Item = &'a mut T;
    type IntoIter = std::collections::vec_deque::IterMut<'a, T>;

    fn into_iter(self) -> std::collections::vec_deque::IterMut<'a, T> {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotate_holds_focus_but_permutes_order() {
        let mut r = Ring::new(vec![1, 2, 3]);

        r.rotate(Direction::Forward);
        assert_eq!(r.as_vec(), vec![3, 1, 2]);
        assert_eq!(r.focused(), Some(&3));

        r.rotate(Direction::Backward);
        assert_eq!(r.as_vec(), vec![1, 2, 3]);
        assert_eq!(r.focused(), Some(&1));
    }

    #[test]
    fn dragging_an_element_forward() {
        let mut r = Ring::new(vec![1, 2, 3, 4]);
        assert_eq!(r.focused(), Some(&1));

        assert_eq!(r.drag_focused(Direction::Forward), Some(&1));
        assert_eq!(r.elements, vec![2, 1, 3, 4]);

        assert_eq!(r.drag_focused(Direction::Forward), Some(&1));
        assert_eq!(r.elements, vec![2, 3, 1, 4]);

        assert_eq!(r.drag_focused(Direction::Forward), Some(&1));
        assert_eq!(r.elements, vec![2, 3, 4, 1]);

        assert_eq!(r.drag_focused(Direction::Forward), Some(&1));
        assert_eq!(r.elements, vec![1, 2, 3, 4]);

        assert_eq!(r.focused(), Some(&1));
    }

    #[test]
    fn dragging_an_element_backward() {
        let mut r = Ring::new(vec![1, 2, 3, 4]);
        assert_eq!(r.focused(), Some(&1));

        assert_eq!(r.drag_focused(Direction::Backward), Some(&1));
        assert_eq!(r.elements, vec![2, 3, 4, 1]);

        assert_eq!(r.drag_focused(Direction::Backward), Some(&1));
        assert_eq!(r.elements, vec![2, 3, 1, 4]);

        assert_eq!(r.drag_focused(Direction::Backward), Some(&1));
        assert_eq!(r.elements, vec![2, 1, 3, 4]);

        assert_eq!(r.drag_focused(Direction::Backward), Some(&1));
        assert_eq!(r.elements, vec![1, 2, 3, 4]);

        assert_eq!(r.focused(), Some(&1));
    }

    #[test]
    fn remove_focused() {
        let mut r = Ring::new(vec![1, 2, 3]);
        r.focused = 2;
        assert_eq!(r.focused(), Some(&3));
        assert_eq!(r.remove(&Selector::Focused), Some(3));
        assert_eq!(r.focused_index(), 1);
        assert_eq!(r.focused(), Some(&2));
        assert_eq!(r.remove(&Selector::Focused), Some(2));
        assert_eq!(r.focused(), Some(&1));
        assert_eq!(r.remove(&Selector::Focused), Some(1));
        assert_eq!(r.focused(), None);
        assert_eq!(r.remove(&Selector::Focused), None);
    }

    #[test]
    fn indices_are_in_bounds() {
        let r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(r.index(&Selector::Index(2)), Some(2));
        assert_eq!(r.index(&Selector::Index(42)), None);
    }

    #[test]
    fn remove() {
        let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        r.focused = 3;
        assert_eq!(r.focused(), Some(&4));
        assert_eq!(r.remove(&Selector::Condition(&|e| e % 2 == 0)), Some(2));
        assert_eq!(r.focused(), Some(&5));
    }

    #[test]
    fn focus() {
        let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(r.focused, 0);
        assert_eq!(
            r.focus(&Selector::Condition(&|e| e % 2 == 0)),
            Some((true, &2)) // focus point updated
        );
        assert_eq!(
            r.focus(&Selector::Condition(&|e| e % 2 == 0)),
            Some((false, &2)) // no focus change this time
        );
        assert_eq!(r.focus(&Selector::Condition(&|e| e % 7 == 0)), None);
    }

    #[test]
    fn cycle_focus() {
        let mut r = Ring::new(vec![1, 2, 3]);
        assert_eq!(r.cycle_focus(Direction::Forward), Some(&2));
        assert_eq!(r.as_vec(), vec![1, 2, 3]);
        assert_eq!(r.cycle_focus(Direction::Backward), Some(&1));
        assert_eq!(r.as_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn element() {
        let mut r = Ring::new(vec![1, 2, 3, 4]);
        assert_eq!(r.element(&Selector::Condition(&|e| e % 2 == 0)), Some(&2));
        assert_eq!(
            r.element_mut(&Selector::Condition(&|e| e % 2 == 0)),
            Some(&mut 2)
        );

        assert_eq!(r.element(&Selector::Index(2)), Some(&3));
        assert_eq!(r.element_mut(&Selector::Index(2)), Some(&mut 3));
        assert_eq!(r.element(&Selector::Index(50)), None);
        assert_eq!(r.element_mut(&Selector::Index(50)), None);

        r.focus(&Selector::Index(1));
        assert_eq!(r.element(&Selector::Focused), Some(&2));
        assert_eq!(r.element_mut(&Selector::Focused), Some(&mut 2));

        assert_eq!(r.element(&Selector::WinId(42)), None);
        assert_eq!(r.element_mut(&Selector::WinId(69)), None);

        assert_eq!(r.as_vec(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn indexed_element() {
        let r = Ring::new(vec![2, 3, 5, 7, 11]);
        assert_eq!(r.indexed_element(&Selector::Focused), Some((0, &2)));
        assert_eq!(r.indexed_element(&Selector::Index(3)), Some((3, &7)));
        assert_eq!(
            r.indexed_element(&Selector::Condition(&|n| n % 5 == 0)),
            Some((2, &5))
        );
    }

    #[test]
    fn all_elements() {
        let mut r = Ring::new(vec![1, 2, 3, 4]);
        assert_eq!(
            r.all_elements(&Selector::Condition(&|e| e % 2 == 0)),
            vec![&2, &4]
        );
        assert_eq!(
            r.all_elements_mut(&Selector::Condition(&|e| e % 2 == 0)),
            vec![&mut 2, &mut 4]
        );

        assert_eq!(r.all_elements(&Selector::Index(2)), vec![&3]);
        assert_eq!(r.all_elements_mut(&Selector::Index(2)), vec![&mut 3]);
        assert_eq!(r.all_elements(&Selector::Index(50)), vec![&0; 0]);
        assert_eq!(r.all_elements_mut(&Selector::Index(50)), vec![&0; 0]);

        r.focus(&Selector::Index(1));
        assert_eq!(r.all_elements(&Selector::Focused), vec![&2]);
        assert_eq!(r.all_elements_mut(&Selector::Focused), vec![&mut 2]);

        assert_eq!(r.all_elements(&Selector::WinId(42)), vec![&0; 0]);
        assert_eq!(r.all_elements_mut(&Selector::WinId(69)), vec![&0; 0]);

        assert_eq!(r.as_vec(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn insert_points() {
        let mut r = Ring::new(vec![0, 0]);
        r.insert_at(&InsertPoint::First, 1);
        assert_eq!(r.as_vec(), vec![1, 0, 0]);
        r.insert_at(&InsertPoint::Last, 2);
        assert_eq!(r.as_vec(), vec![1, 0, 0, 2]);
        r.insert_at(&InsertPoint::Index(3), 3);
        assert_eq!(r.as_vec(), vec![1, 0, 0, 3, 2]);
        r.focus(&Selector::Index(1));
        r.insert_at(&InsertPoint::Focused, 4);
        assert_eq!(r.as_vec(), vec![1, 4, 0, 0, 3, 2]);
        r.insert_at(&InsertPoint::AfterFocused, 5);
        assert_eq!(r.as_vec(), vec![1, 4, 5, 0, 0, 3, 2]);
        r.focus(&Selector::Index(6));
        r.insert_at(&InsertPoint::AfterFocused, 6);
        assert_eq!(r.as_vec(), vec![1, 4, 5, 0, 0, 3, 2, 6]);
    }

    #[test]
    fn vec_map() {
        let contents = vec!["this", "is", "a", "lot", "nicer"];
        let r = Ring::new(contents.clone());
        let lens = r.vec_map(|s| s.len());
        assert_eq!(lens, vec![4, 2, 1, 3, 5]);
        assert_eq!(r.as_vec(), contents);
    }

    #[test]
    fn apply_to() {
        let contents = vec!["original", "original", "original"];
        let mut r = Ring::new(contents.clone());
        r.apply_to(&Selector::Index(2), |s| *s = "mutated");
        assert_eq!(r.as_vec(), vec!["original", "original", "mutated"]);
    }
}
