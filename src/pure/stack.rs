use crate::pop_where;
use std::{
    collections::linked_list::{self, LinkedList},
    fmt,
    iter::IntoIterator,
    mem::{swap, take},
};

/// Create a [Stack] containing the arguments. The only required element is the focus,
/// it is not possible to create an empty [Stack];
/// ```
/// # use penrose::stack;
/// let s = stack!([1, 2], 3, [4, 5]);
/// let s = stack!([1, 2], 3);
/// let s = stack!(1, [2, 3]);
/// let s = stack!(1, 2, 3);
/// let s = stack!(1);
/// ```
#[macro_export]
macro_rules! stack {
    ([$($up:expr),*], $focus:expr, [$($down:expr),*]) => { $crate::Stack::new([$($up),*], $focus, [$($down),*]) };
    ([$($up:expr),*], $focus:expr) => { $crate::Stack::new([$($up),*], $focus, []) };
    ($focus:expr, [$($down:expr),*]) => { $crate::Stack::new([], $focus, [$($down),*]) };
    ($focus:expr, $($down:expr),+) => { $crate::Stack::new([], $focus, [$($down),*]) };
    ($focus:expr) => { $crate::Stack::new([], $focus, []) };
}

/// A position within a [Stack].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Position {
    /// The current focus point
    Focus,
    /// Above the current focus point
    Before,
    /// Below the current focus point
    After,
    /// The first element of the stack
    Head,
    /// The last element of the stack
    Tail,
}

impl Default for Position {
    fn default() -> Self {
        Position::Focus
    }
}

/// A [Stack] can be thought of as a [LinkedList] with a hole punched in it to mark
/// a single element that currently holds focus. By convention, the main element is
/// the first element in the stack (regardless of focus). Focusing operations do not
/// reorder the elements of the stack or the resulting [Vec] that can be obtained
/// from calling [Stack::flatten].
///
/// This is a [zipper](https://en.wikipedia.org/wiki/Zipper_(data_structure))
/// over a [LinkedList]. Many of the methods that mutate the structure of the Stack
/// return back a mutable reference so that they are able to be chained.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stack<T> {
    pub(crate) up: LinkedList<T>,
    pub(crate) focus: T,
    pub(crate) down: LinkedList<T>,
}

impl<T: fmt::Display> fmt::Display for Stack<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let up: Vec<String> = self.up.iter().rev().map(|t| format!("{t}")).collect();
        let down: Vec<String> = self.down.iter().map(|t| format!("{t}")).collect();

        write!(
            f,
            "Stack([{}], {}, [{}])",
            up.join(", "),
            self.focus,
            down.join(", ")
        )
    }
}

impl<T> Stack<T> {
    /// Create a new Stack specifying the focused element and and elements
    /// above and below it.
    pub fn new<I, J>(up: I, focus: T, down: J) -> Self
    where
        I: IntoIterator<Item = T>,
        J: IntoIterator<Item = T>,
    {
        let mut reversed_up = LinkedList::new();
        for elem in up.into_iter() {
            reversed_up.push_front(elem);
        }

        Self {
            focus,
            up: reversed_up,
            down: down.into_iter().collect(),
        }
    }

    // NOTE: Can't implement FromIterator<T> because we disallow an empty stack
    /// For an iterator of at least one element, the first element will
    /// be focused and all remaining elements will be placed after it.
    /// For an empty iterator, return None.
    pub fn try_from_iter<I>(iter: I) -> Option<Self>
    where
        I: IntoIterator<Item = T>,
    {
        let mut it = iter.into_iter();

        let focus = match it.next() {
            Some(t) => t,
            None => return None,
        };

        Some(Self {
            up: LinkedList::default(),
            focus,
            down: it.collect(),
        })
    }

    pub(crate) fn from_iter_unchecked<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut it = iter.into_iter();
        let focus = it.next().expect("at least one element");

        Self {
            up: LinkedList::default(),
            focus,
            down: it.collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.up.len() + self.down.len() + 1
    }

    /// Always false: a Stack always has at least one focused element
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Provide an iterator over this stack iterating over up,
    /// focus and then down.
    pub fn iter(&self) -> Iter<T> {
        Iter {
            up: self.up.iter(),
            focus: Some(&self.focus),
            down: self.down.iter(),
        }
    }

    /// Provide an iterator over this stack iterating over up,
    /// focus and then down with mutable references.
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            up: self.up.iter_mut(),
            focus: Some(&mut self.focus),
            down: self.down.iter_mut(),
        }
    }

    /// Flatten a Stack into a Vector, losing the information of which
    /// element is focused.
    pub fn flatten(self) -> Vec<T> {
        self.into_iter().collect()
    }

    /// Flatten a Stack into a Vector, losing the information of which
    /// element is focused.
    /// (Alias of `flatten`)
    pub fn integrate(self) -> Vec<T> {
        self.into_iter().collect()
    }

    /// Turn an iterable into a possibly empty Stack with the first
    /// element focused and remaining elements placed after it.
    /// (Alias of `try_from_iter`)
    pub fn differentiate<I>(iter: I) -> Option<Self>
    where
        I: IntoIterator<Item = T>,
    {
        Self::try_from_iter(iter)
    }

    /// Return a reference to the first element in this [Stack]
    pub fn head(&self) -> &T {
        self.up.back().unwrap_or(&self.focus)
    }

    /// Return a reference to the focused element in this [Stack]
    pub fn focused(&self) -> &T {
        &self.focus
    }

    /// Return a reference to the last element in this [Stack]
    pub fn last(&self) -> &T {
        self.down.back().unwrap_or(&self.focus)
    }

    /// Swap the current head element with the focused element in the
    /// stack order. Focus stays with the original focused element.
    pub fn swap_focus_and_head(&mut self) -> &mut Self {
        let mut tmp = take(&mut self.up);

        if let Some(head) = tmp.pop_back() {
            self.down.push_front(head);
        }

        for item in tmp.into_iter() {
            self.down.push_front(item);
        }

        self
    }

    /// Rotate the Stack until the current focused element is in the head position
    pub fn rotate_focus_to_head(&mut self) -> &mut Self {
        if self.up.is_empty() {
            return self;
        }

        for item in take(&mut self.up).into_iter().rev() {
            self.down.push_back(item);
        }

        self
    }

    pub fn focus_head(&mut self) -> &mut Self {
        let mut head = match self.up.pop_back() {
            None => return self, // focus is already head
            Some(t) => t,
        };

        swap(&mut head, &mut self.focus);
        self.down.push_front(head);

        for item in take(&mut self.up).into_iter().rev() {
            self.down.push_front(item);
        }

        self
    }

    /// Insert the given element in place of the current focus, pushing
    /// the current focus down the [Stack].
    pub fn insert(&mut self, t: T) -> &mut Self {
        self.insert_at(Position::default(), t)
    }

    /// Insert the given element at the requested position in the [Stack].
    /// See [Position] for the semantics of each case. For all cases, the
    /// existing elements in the [Stack] are pushed down to make room for
    /// the new one.
    pub fn insert_at(&mut self, pos: Position, mut t: T) -> &mut Self {
        use Position::*;

        match pos {
            Focus => {
                self.swap_focus(&mut t);
                self.down.push_front(t);
            }
            Before => self.up.push_front(t),
            After => self.down.push_front(t),
            Head => self.up.push_back(t),
            Tail => self.down.push_back(t),
        };

        self
    }

    /// Remove the focused element of this Stack. If this was the only element then
    /// the stack is dropped and None is returned.
    pub fn remove_focused(mut self) -> (T, Option<Self>) {
        let focus = match self.down.pop_front().or_else(|| self.up.pop_front()) {
            Some(focus) => focus,
            None => return (self.focus, None),
        };

        (
            self.focus,
            Some(Self {
                focus,
                up: self.up,
                down: self.down,
            }),
        )
    }

    /// Remove an element from the stack.
    ///
    /// If the element was present it is returned along with the rest of the [Stack].
    /// If this was the last element in the stack, the stack is dropped and None is
    /// returned.
    pub fn remove(mut self, t: &T) -> (Option<T>, Option<Self>)
    where
        T: PartialEq,
    {
        if let Some(found) = pop_where!(self, up, |elem: &T| elem == t) {
            return (Some(found), Some(self));
        }

        if let Some(found) = pop_where!(self, down, |elem: &T| elem == t) {
            return (Some(found), Some(self));
        }

        if t == &self.focus {
            let (focus, stack) = self.remove_focused();
            (Some(focus), stack)
        } else {
            (None, Some(self))
        }
    }

    /// Map a function over all elements in this [Stack], returning a new one.
    pub fn map<F, U>(self, f: F) -> Stack<U>
    where
        F: Fn(T) -> U,
    {
        Stack {
            focus: f(self.focus),
            up: self.up.into_iter().map(&f).collect(),
            down: self.down.into_iter().map(&f).collect(),
        }
    }

    /// Retain only elements which satisfy the given predicate. If the focused
    /// element is removed then focus shifts to the first remaining element
    /// after it, if there are no elements after then focus moves to the first
    /// remaining element before. If no elements satisfy the predicate then
    /// None is returned.
    pub fn filter<F>(self, f: F) -> Option<Self>
    where
        F: Fn(&T) -> bool,
    {
        let new_stack = Self {
            focus: self.focus,
            up: self.up.into_iter().filter(&f).collect(),
            down: self.down.into_iter().filter(&f).collect(),
        };

        if f(&new_stack.focus) {
            Some(new_stack)
        } else {
            let (_, maybe_stack) = new_stack.remove_focused();
            maybe_stack
        }
    }

    /// Reverse the ordering of a Stack (up becomes down) while maintaining
    /// focus.
    pub fn reverse(&mut self) -> &mut Self {
        swap(&mut self.up, &mut self.down);

        self
    }

    pub(crate) fn swap_focus(&mut self, new: &mut T) {
        swap(&mut self.focus, new);
    }

    fn rev_up(&mut self) -> &mut Self {
        let mut reversed = take(&mut self.up).into_iter().rev().collect();
        swap(&mut self.up, &mut reversed);

        self
    }

    fn rev_down(&mut self) -> &mut Self {
        let mut reversed = take(&mut self.down).into_iter().rev().collect();
        swap(&mut self.down, &mut reversed);

        self
    }

    /// Move focus from the current element up the stack, wrapping to the
    /// bottom if focus is already at the top.
    pub fn focus_up(&mut self) -> &mut Self {
        match (self.up.is_empty(), self.down.is_empty()) {
            // xs:x f ys   -> xs x f:ys
            // xs:x f []   -> xs x f
            (false, _) => {
                let mut focus = self.up.pop_front().expect("non-empty");
                self.swap_focus(&mut focus);
                self.down.push_front(focus);
            }

            // [] f ys:y   -> f:ys y []
            (true, false) => {
                let mut focus = self.down.pop_back().expect("non-empty");
                self.swap_focus(&mut focus);
                self.down.push_front(focus);
                self.reverse().rev_up();
            }

            // [] f []     -> [] f []
            (true, true) => (),
        }

        self
    }

    /// Move focus from the current element down the stack, wrapping to the
    /// top if focus is already at the bottom.
    pub fn focus_down(&mut self) -> &mut Self {
        match (self.up.is_empty(), self.down.is_empty()) {
            // xs f y:ys   -> xs:f y ys
            // [] f y:ys   -> f y ys
            (_, false) => {
                let mut focus = self.down.pop_front().expect("non-empty");
                self.swap_focus(&mut focus);
                self.up.push_front(focus);
            }

            // x:xs f []   -> [] x xs:f
            (false, true) => {
                let mut focus = self.up.pop_back().expect("non-empty");
                self.swap_focus(&mut focus);
                self.up.push_front(focus);
                self.reverse().rev_down();
            }

            // [] f []     -> [] f []
            (true, true) => (),
        }

        self
    }

    /// Swap the focused element with the one above, wrapping from top to bottom.
    /// The currently focused element is maintained by this operation.
    pub fn swap_up(&mut self) -> &mut Self {
        match self.up.pop_front() {
            Some(t) => {
                self.down.push_front(t);
                self
            }
            None => self.reverse().rev_up(),
        }
    }

    /// Swap the focused element with the one below, wrapping from top to bottom.
    /// The currently focused element is maintained by this operation.
    pub fn swap_down(&mut self) -> &mut Self {
        match self.down.pop_front() {
            Some(t) => {
                self.up.push_front(t);
                self
            }
            None => self.reverse().rev_down(),
        }
    }

    /// Rotate all elements of the stack forward, wrapping from top to bottom.
    /// The currently focused element in the stack is maintained by this operation.
    pub fn rotate_up(&mut self) -> &mut Self {
        match self.up.pop_back() {
            Some(t) => {
                self.down.push_back(t);
                self
            }
            None => self.reverse().rev_up(),
        }
    }

    /// Rotate all elements of the stack back, wrapping from bottom to top.
    /// The currently focused element in the stack is maintained by this operation.
    pub fn rotate_down(&mut self) -> &mut Self {
        match self.down.pop_back() {
            Some(t) => {
                self.up.push_back(t);
                self
            }
            None => self.reverse().rev_down(),
        }
    }
}

impl<T: Clone> Stack<T> {
    pub fn from_filtered<F>(&self, f: F) -> Option<Self>
    where
        F: Fn(&T) -> bool,
    {
        let new = self.clone();

        new.filter(f)
    }
}

impl<T: PartialEq> Stack<T> {
    pub fn contains(&self, t: &T) -> bool {
        &self.focus == t || self.up.contains(t) || self.down.contains(t)
    }
}

// Iteration

#[derive(Debug)]
pub struct IntoIter<T> {
    focus: Option<T>,
    up: LinkedList<T>,
    down: LinkedList<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        self.up
            .pop_back()
            .or_else(|| self.focus.take())
            .or_else(|| self.down.pop_front())
    }
}

impl<T> IntoIterator for Stack<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> IntoIter<T> {
        IntoIter {
            focus: Some(self.focus),
            up: self.up,
            down: self.down,
        }
    }
}

#[derive(Debug)]
pub struct Iter<'a, T> {
    up: linked_list::Iter<'a, T>,
    focus: Option<&'a T>,
    down: linked_list::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.up
            .next_back()
            .or_else(|| self.focus.take())
            .or_else(|| self.down.next())
    }
}

impl<'a, T> IntoIterator for &'a Stack<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

#[derive(Debug)]
pub struct IterMut<'a, T> {
    focus: Option<&'a mut T>,
    up: linked_list::IterMut<'a, T>,
    down: linked_list::IterMut<'a, T>,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.up
            .next_back()
            .or_else(|| self.focus.take())
            .or_else(|| self.down.next())
    }
}

impl<'a, T> IntoIterator for &'a mut Stack<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;

    fn into_iter(self) -> IterMut<'a, T> {
        self.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use simple_test_case::test_case;

    #[test]
    fn focused() {
        let s = stack!([1, 2], 3, [4, 5]);

        assert_eq!(s.focused(), &3)
    }

    #[test]
    fn head() {
        let s = stack!([1, 2], 3, [4, 5]);

        assert_eq!(s.head(), &1)
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!(3, [2, 1, 4, 5]); "items up and down")]
    #[test_case(stack!([1, 2], 3), stack!(3, [2, 1]); "items up")]
    #[test_case(stack!(3, [4, 5]), stack!(3, [4, 5]); "items down")]
    #[test_case(stack!(3), stack!(3); "focus only")]
    #[test]
    fn swap_focus_and_head(mut s: Stack<u8>, expected: Stack<u8>) {
        s.swap_focus_and_head();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!(3, [4, 5, 1, 2]); "items up and down")]
    #[test_case(stack!([1, 2], 3), stack!(3, [1, 2]); "items up")]
    #[test_case(stack!(3, [4, 5]), stack!(3, [4, 5]); "items down")]
    #[test_case(stack!(3), stack!(3); "focus only")]
    #[test]
    fn rotate_focus_to_head(mut s: Stack<u8>, expected: Stack<u8>) {
        s.rotate_focus_to_head();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!(1, [2, 3, 4, 5]); "items up and down")]
    #[test_case(stack!([1, 2], 3), stack!(1, [2, 3]); "items up")]
    #[test_case(stack!(3, [4, 5]), stack!(3, [4, 5]); "items down")]
    #[test_case(stack!(3), stack!(3); "focus only")]
    #[test]
    fn focus_head(mut s: Stack<u8>, expected: Stack<u8>) {
        s.focus_head();

        assert_eq!(s, expected);
    }

    #[test]
    fn iter_yields_all_elements_in_order() {
        let s = stack!([1, 2], 3, [4, 5]);
        let elems: Vec<u8> = s.iter().copied().collect();

        assert_eq!(elems, vec![1, 2, 3, 4, 5])
    }

    #[test]
    fn iter_mut_yields_all_elements_in_order() {
        let mut s = stack!([1, 2], 3, [4, 5]);
        let elems: Vec<u8> = s.iter_mut().map(|c| *c).collect();

        assert_eq!(elems, vec![1, 2, 3, 4, 5])
    }

    #[test]
    fn into_iter_yields_all_elements_in_order() {
        let s = stack!([1, 2], 3, [4, 5]);
        let elems: Vec<u8> = s.into_iter().collect();

        assert_eq!(elems, vec![1, 2, 3, 4, 5])
    }

    #[test]
    fn map_preserves_structure() {
        let s = stack!(["a", "bunch"], "of", ["string", "refs"]);

        let mapped = s.map(|x| x.len());
        let expected = stack!([1, 5], 2, [6, 4]);

        assert_eq!(mapped, expected);
    }

    #[test_case(|&x| x > 5, None; "returns None if no elements satisfy the predicate")]
    #[test_case(|x| x % 2 == 1, Some(stack!([3], 1, [5])); "holds focus with predicate")]
    #[test_case(|x| x % 2 == 0, Some(stack!([2], 4)); "moves focus to top of down when possible")]
    #[test_case(|&x| x == 2 || x == 3, Some(stack!([2], 3)); "moves focus to end of up if down is empty")]
    #[test]
    fn filter(predicate: fn(&usize) -> bool, expected: Option<Stack<usize>>) {
        let mapped = stack!([2, 3], 1, [4, 5]).filter(predicate);

        assert_eq!(mapped, expected);
    }

    #[test]
    fn integrate_is_correctly_ordered() {
        let res = stack!([1, 2], 3, [4, 5]).integrate();

        assert_eq!(res, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn differentiate_is_correctly_ordered() {
        let res = Stack::differentiate(vec![1, 2, 3, 4, 5]);

        assert_eq!(res, Some(stack!(1, [2, 3, 4, 5])));
    }

    #[test]
    fn differentiate_of_empty_iterable_is_none() {
        let empty: Vec<()> = vec![];

        assert_eq!(Stack::differentiate(empty), None);
    }

    #[test]
    fn int_diff_with_empty_up_is_inverse() {
        let s = stack!(1, [2, 3, 4]);
        let res = Stack::differentiate(s.clone().integrate());

        assert_eq!(res, Some(s));
    }

    #[test]
    fn reverse_holds_focus() {
        let mut s = stack!([1, 2], 3, [4, 5]);
        s.reverse();

        assert_eq!(s, stack!([5, 4], 3, [2, 1]));
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([1], 2, [3, 4, 5]); "items up and down")]
    #[test_case(stack!([], 1, [2, 3]), stack!([1, 2], 3); "items down only")]
    #[test_case(stack!([1, 2], 3, []), stack!([1], 2, [3]); "items up only")]
    #[test_case(stack!([], 1, []), stack!(1); "only focused")]
    #[test]
    fn focus_up(mut s: Stack<usize>, expected: Stack<usize>) {
        s.focus_up();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([1, 2, 3], 4, [5]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!([1], 2, [3]); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!(1, [2, 3]); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn focus_down(mut s: Stack<usize>, expected: Stack<usize>) {
        s.focus_down();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([1], 3, [2, 4, 5]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!([2, 3], 1); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!([1], 3, [2]); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn swap_up(mut s: Stack<usize>, expected: Stack<usize>) {
        s.swap_up();

        assert_eq!(s, expected);
    }

    #[test]
    fn swap_up_chained() {
        let mut s = stack!([1, 2], 3, [4]);

        s.swap_up();
        assert_eq!(s, stack!([1], 3, [2, 4]));
        s.swap_up();
        assert_eq!(s, stack!(3, [1, 2, 4]));
        s.swap_up();
        assert_eq!(s, stack!([1, 2, 4], 3));
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([1, 2, 4], 3, [5]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!([2], 1, [3]); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!(3, [1, 2]); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn swap_down(mut s: Stack<usize>, expected: Stack<usize>) {
        s.swap_down();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([2], 3, [4, 5, 1]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!([2, 3], 1); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!([2], 3, [1]); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn rotate_up(mut s: Stack<usize>, expected: Stack<usize>) {
        s.rotate_up();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([5, 1, 2], 3, [4]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!([3], 1, [2]); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!(3, [1, 2]); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn rotate_down(mut s: Stack<usize>, expected: Stack<usize>) {
        s.rotate_down();

        assert_eq!(s, expected);
    }

    #[test_case(Position::Focus, stack!([1,2], 6, [3,4,5]); "focus")]
    #[test_case(Position::Before, stack!([1,2,6], 3, [4,5]); "before")]
    #[test_case(Position::After, stack!([1,2], 3, [6,4,5]); "after")]
    #[test_case(Position::Head, stack!([6,1,2], 3, [4,5]); "head")]
    #[test_case(Position::Tail, stack!([1,2], 3, [4,5,6]); "tail")]
    #[test]
    fn insert_at(pos: Position, expected: Stack<usize>) {
        let mut s = stack!([1, 2], 3, [4, 5]);
        s.insert_at(pos, 6);

        assert_eq!(s, expected);
    }
}

#[cfg(test)]
mod quickcheck_tests {
    use super::*;
    use quickcheck::{Arbitrary, Gen};
    use quickcheck_macros::quickcheck;
    use std::collections::HashSet;

    // For the tests below we only care about the stack structure not the elements themselves, so
    // we use `u8` as an easily defaultable focus if `Vec::arbitrary` gives us an empty vec.
    //
    // Focus is always `42` and elements are unique.
    impl Arbitrary for Stack<u8> {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut up: Vec<u8> = HashSet::<u8>::arbitrary(g)
                .into_iter()
                .filter(|&n| n != 42)
                .collect();

            let focus = 42;

            if up.is_empty() {
                return stack!(focus); // return a minimal stack as we don't allow empty
            }

            let split_at = usize::arbitrary(g) % (up.len());
            let down = up.split_off(split_at);

            Self::new(up, focus, down)
        }
    }

    impl<T> Stack<T> {
        // Helper to reduce the verbosity of some of the composition laws
        fn rev_both(&mut self) -> &mut Self {
            self.rev_up().rev_down()
        }
    }

    #[quickcheck]
    fn remove_and_re_insert_is_idempotent(stack: Stack<u8>) -> bool {
        let original = stack.clone();
        let focused = *stack.focused();

        match stack.remove(&focused) {
            (Some(t), Some(mut s)) => {
                s.insert(t);
                s == original
            }

            (Some(t), None) => stack!(t) == original,

            _ => panic!("delete of focused returned None"),
        }
    }

    // Define a composition law for operations on a Stack.
    // Using these as the real implementation is not particularly efficient but the laws should
    // hold for the hand written impls as well.
    macro_rules! composition_law {
        ($test:ident => $method:ident == $($f:ident).+) => {
            #[quickcheck]
            fn $test(mut stack: Stack<u8>) -> bool {
                let mut by_composition = stack.clone();
                by_composition.$($f()).+;
                stack.$method();

                stack == by_composition
            }
        }
    }

    composition_law!(
        focus_down_from_focus_up =>
        focus_down == reverse . focus_up . reverse
    );

    composition_law!(
        swap_down_from_swap_up =>
        swap_down == reverse . swap_up . reverse
    );

    composition_law!(
        rotate_up_from_swap_up =>
        rotate_up == rev_both . swap_up . rev_both
    );

    composition_law!(
        rotate_down_from_swap_up =>
        rotate_down == rev_both . reverse . swap_up . reverse . rev_both
    );

    composition_law!(
        rotate_down_from_rotate_up =>
        rotate_down == reverse . rotate_up . reverse
    );

    // Two methods that should act as both left and right inverses of one another
    macro_rules! are_inverse {
        ($test:ident => $a:ident <> $b:ident) => {
            paste::paste! {
                #[quickcheck]
                fn [<inverse _ $test _ left_right>](mut stack: Stack<u8>) -> bool {
                    let original = stack.clone();
                    stack.$a().$b();

                    stack == original
                }

                #[quickcheck]
                fn [<inverse _ $test _ right_left>](mut stack: Stack<u8>) -> bool {
                    let original = stack.clone();
                    stack.$b().$a();

                    stack == original
                }
            }
        };
    }

    are_inverse!(reverse  => reverse   <> reverse);
    are_inverse!(rev_up   => rev_up    <> rev_up);
    are_inverse!(rev_down => rev_down  <> rev_down);
    are_inverse!(focus    => focus_up  <> focus_down);
    are_inverse!(swap     => swap_up   <> swap_down);
    are_inverse!(rotate   => rotate_up <> rotate_down);
}
