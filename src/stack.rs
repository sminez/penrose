use std::{
    collections::linked_list::{self, LinkedList},
    iter::IntoIterator,
};

/// Create a [Stack] containing the arguments. The only required element is the focus,
/// it is not possible to create an empty [Stack];
/// ```
/// # use penrose::stack;
/// let s = stack!([1, 2], 3, [4, 5]);
/// let s = stack!([1, 2], 3);
/// let s = stack!(1, [2, 3]);
/// let s = stack!(1);
/// ```
#[macro_export]
macro_rules! stack {
    ([$($up:expr),*], $focus:expr, [$($down:expr),*]) => { $crate::Stack::new([$($up),*], $focus, [$($down),*]) };
    ([$($up:expr),*], $focus:expr) => { $crate::Stack::new([$($up),*], $focus, []) };
    ($focus:expr, [$($down:expr),*]) => { $crate::Stack::new([], $focus, [$($down),*]) };
    ($focus:expr) => { $crate::Stack::new([], $focus, []) };
}

// Helper for reversing a linked list in place
macro_rules! rev {
    ($self:ident, $lst:ident) => {
        let mut placeholder = LinkedList::default();
        std::mem::swap(&mut $self.$lst, &mut placeholder);
        let mut reversed = placeholder.into_iter().rev().collect();
        std::mem::swap(&mut $self.$lst, &mut reversed);
    };
}

// TODO: xmonad store the `up` list in reverse order of the integral like so,
//   ([2, 1], 3, [4, 5]) -> [1, 2, 3, 4, 5]
// This gives better performance for the reordering operations as we just pop
// the head of the list rather than traversing all nodes. It does lead to a more
// confusing public API though if I want to be able to make the fields public for
// users to interact with at some point.
//
// If performance looks off, it'd be worth reworking the methods that manipulate
// `up` to store the list in reverse order but I suspect for the size of the lists
// in question the difference is going to be minimal in practice.

/// A [Stack] can be thought of as a [LinkedList] with a hole punched in it to mark
/// a single element that currently holds focus. By convention, the main element is
/// the first element in the stack (regardless of focus). Focusing operations do not
/// reorder the elements of the stack or the resulting [Vec] that can be obtained
/// from calling [Stack::flatten].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stack<T> {
    focus: T,
    up: LinkedList<T>,
    down: LinkedList<T>,
}

impl<T> Stack<T> {
    /// Create a new Stack specifying the focused element and and elements
    /// above and below it.
    pub fn new<I, J>(up: I, focus: T, down: J) -> Self
    where
        I: IntoIterator<Item = T>,
        J: IntoIterator<Item = T>,
    {
        Self {
            focus,
            up: up.into_iter().collect(),
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
            focus,
            up: LinkedList::default(),
            down: it.collect(),
        })
    }

    /// Provide an iterator over this stack iterating over up,
    /// focus and then down.
    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter {
            focus: Some(&self.focus),
            up: self.up.iter(),
            down: self.down.iter(),
        }
    }

    /// Provide an iterator over this stack iterating over up,
    /// focus and then down with mutable references.
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
        IterMut {
            focus: Some(&mut self.focus),
            up: self.up.iter_mut(),
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
        self.up.front().unwrap_or(&self.focus)
    }

    /// Return a reference to the focused element in this [Stack]
    pub fn focused(&self) -> &T {
        &self.focus
    }

    /// Map a function over all elements in this [Stack], returning a new one.
    pub fn map<U>(self, f: fn(T) -> U) -> Stack<U> {
        Stack {
            focus: f(self.focus),
            up: self.up.into_iter().map(f).collect(),
            down: self.down.into_iter().map(f).collect(),
        }
    }

    /// Retain only elements which satisfy the given predicate. If the focused
    /// element is removed then focus shifts to the first remaining element
    /// after it, if there are no elements after then focus moves to the first
    /// remaining element before. If no elements satisfy the predicate then
    /// None is returned.
    pub fn filter(self, f: fn(&T) -> bool) -> Option<Self> {
        let mut up: LinkedList<T> = self.up.into_iter().filter(f).collect();
        let mut down: LinkedList<T> = self.down.into_iter().filter(f).collect();

        let focus = if f(&self.focus) {
            self.focus
        } else {
            match down.pop_front().or_else(|| up.pop_back()) {
                Some(focus) => focus,
                None => return None,
            }
        };

        Some(Self { focus, up, down })
    }

    /// Reverse the ordering of a Stack (up becomes down) while maintaining
    /// focus.
    pub fn reverse(&mut self) {
        rev!(self, up);
        rev!(self, down);
        self.swap_up_down();
    }

    fn swap_up_down(&mut self) {
        std::mem::swap(&mut self.up, &mut self.down);
    }

    /// Move focus from the current element up the stack, wrapping to the
    /// bottom if focus is already at the top.
    pub fn focus_up(&mut self) {
        match (self.up.is_empty(), self.down.is_empty()) {
            // xs:x f ys   -> xs x f:ys
            // xs:x f []   -> xs x f
            (false, _) => {
                let mut focus = self.up.pop_back().expect("non-empty");
                std::mem::swap(&mut self.focus, &mut focus);
                self.down.push_front(focus);
            }

            // [] f ys:y   -> f:ys y []
            (true, false) => {
                let mut focus = self.down.pop_back().expect("non-empty");
                std::mem::swap(&mut self.focus, &mut focus);
                self.down.push_front(focus);
                self.swap_up_down();
            }

            // [] f []     -> [] f []
            (true, true) => (),
        }
    }

    /// Move focus from the current element down the stack, wrapping to the
    /// top if focus is already at the bottom.
    pub fn focus_down(&mut self) {
        self.reverse();
        self.focus_up();
        self.reverse();
    }

    // NOTE: xmonad calls this swap_up?
    /// Rotate all elements of the stack forward, wrapping from top to bottom.
    /// The currently focused element is maintained by this operation.
    pub fn rotate_up(&mut self) {
        match self.up.pop_front() {
            Some(t) => self.down.push_back(t),
            None => self.swap_up_down(),
        }
    }

    // NOTE: xmonad calls this swap_down?
    /// Rotate all elements of the stack forward, wrapping from top to bottom.
    /// The currently focused element is maintained by this operation.
    pub fn rotate_down(&mut self) {
        match self.down.pop_back() {
            Some(t) => self.up.push_front(t),
            None => self.swap_up_down(),
        }
    }

    /// Rotate all elements of the stack forward, wrapping from top to bottom.
    /// The currently focused position in the stack is maintained by this operation.
    pub fn cycle_up(&mut self) {
        self.focus_down();
        self.rotate_up();
    }

    /// Rotate all elements of the stack back, wrapping from bottom to top.
    /// The currently focused position in the stack is maintained by this operation.
    pub fn cycle_down(&mut self) {
        self.focus_up();
        self.rotate_down();
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
            .pop_front()
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
    focus: Option<&'a T>,
    up: linked_list::Iter<'a, T>,
    down: linked_list::Iter<'a, T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.up.next().or(self.focus).or_else(|| self.down.next())
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
            .next()
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
        let res = stack!([2, 3], 1, [4, 5]).integrate();

        assert_eq!(res, vec![2, 3, 1, 4, 5]);
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

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([2, 3], 4, [5, 1]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!(2, [3, 1]); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!([2, 3], 1); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn cycle_up(mut s: Stack<usize>, expected: Stack<usize>) {
        s.cycle_up();

        assert_eq!(s, expected);
    }

    #[test_case(stack!([1, 2], 3, [4, 5]), stack!([5, 1], 2, [3, 4]); "items up and down")]
    #[test_case(stack!(1, [2, 3]), stack!(3, [1, 2]); "items down only")]
    #[test_case(stack!([1, 2], 3), stack!([3, 1], 2); "items up only")]
    #[test_case(stack!(1), stack!(1); "only focused")]
    #[test]
    fn cycle_down(mut s: Stack<usize>, expected: Stack<usize>) {
        s.cycle_down();

        assert_eq!(s, expected);
    }
}
