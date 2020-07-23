//! Simple data types and enums
use crate::layout::Layout;
use crate::manager::WindowManager;
use std::collections::{HashMap, VecDeque};
use std::ops;
use xcb;

/// Some action to be run by a user key binding
pub type FireAndForget = Box<dyn Fn(&mut WindowManager) -> ()>;

/// User defined key bindings
pub type KeyBindings = HashMap<KeyCode, FireAndForget>;

/// Output of a Layout function: the new position a window should take
pub type ResizeAction = (WinId, Region);

/// Map xmodmap key names to their X key code so that we can bind them by name
pub type CodeMap = HashMap<String, u8>;

/// An X window ID
pub type WinId = u32;

/// An x,y coordinate pair
#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

impl Point {
    pub fn new(x: u32, y: u32) -> Point {
        Point { x, y }
    }
}

/// The main user facing configuration details
#[derive(Debug)]
pub struct Config {
    pub workspaces: &'static [&'static str],
    pub fonts: &'static [&'static str],
    pub floating_classes: &'static [&'static str],
    pub layouts: Vec<Layout>,
    pub color_scheme: ColorScheme,
    pub border_px: u32,
    pub gap_px: u32,
    pub main_ratio_step: f32,
    pub systray_spacing_px: u32,
    pub show_systray: bool,
    pub show_bar: bool,
    pub top_bar: bool,
    pub bar_height: u32,
    pub respect_resize_hints: bool,
}

/* Argument enums */

/// A direction to permute a Ring
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    /// increase the index, wrapping if needed
    Forward,
    /// decrease the index, wrapping if needed
    Backward,
}

impl Direction {
    pub fn reverse(&self) -> Direction {
        match self {
            Direction::Forward => Direction::Backward,
            Direction::Backward => Direction::Forward,
        }
    }
}

/// Increment / decrement a value
#[derive(Debug, Copy, Clone)]
pub enum Change {
    /// increase the value
    More,
    /// decrease the value, possibly clamping
    Less,
}

/// X window border kind
#[derive(Debug)]
pub enum Border {
    /// window is urgent
    Urgent,
    /// window currently has focus
    Focused,
    /// window does not have focus
    Unfocused,
}

/// An X window / screen position: top left corner + extent
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Region {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Region {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Region {
        Region { x, y, w, h }
    }

    pub fn width(&self) -> u32 {
        self.w
    }

    pub fn height(&self) -> u32 {
        self.h
    }

    pub fn values(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }
}

/// A set of named color codes
#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    pub bg: u32,
    pub fg_1: u32,
    pub fg_2: u32,
    pub fg_3: u32,
    pub highlight: u32,
    pub urgent: u32,
}

/// An X key-code along with a modifier mask
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct KeyCode {
    pub mask: u16,
    pub code: u8,
}

impl KeyCode {
    pub fn from_key_press(k: &xcb::KeyPressEvent) -> KeyCode {
        KeyCode {
            mask: k.state(),
            code: k.detail(),
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
#[derive(Debug)]
pub(crate) struct Ring<T> {
    elements: VecDeque<T>,
    focused: usize,
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

    pub fn focused_index(&mut self) -> usize {
        self.focused
    }

    pub fn focused(&self) -> Option<&T> {
        self.elements.get(self.focused)
    }

    pub fn focused_mut(&mut self) -> Option<&mut T> {
        self.elements.get_mut(self.focused)
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

    pub fn focus_nth(&mut self, n: usize) -> Option<&T> {
        self.focused = n;
        self.focused()
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

    pub fn focus_by(&mut self, cond: impl Fn(&T) -> bool) -> Option<&T> {
        if let Some((i, _)) = self.elements.iter().enumerate().find(|(_, e)| cond(*e)) {
            self.focused = i;
            Some(&self.elements[self.focused])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.elements.insert(index, element);
    }

    pub fn remove_by(&mut self, cond: impl Fn(&T) -> bool) -> Option<T> {
        if let Some((i, _)) = self.elements.iter().enumerate().find(|(_, e)| cond(*e)) {
            if self.focused > 0 && self.focused == self.elements.len() - 1 {
                self.focused -= 1;
            }
            self.elements.remove(i)
        } else {
            None
        }
    }

    pub fn remove_focused(&mut self) -> Option<T> {
        if self.elements.len() == 0 {
            return None;
        }

        let c = self.elements.remove(self.focused);
        if self.focused > 0 && self.focused >= self.elements.len() - 1 {
            self.focused -= 1;
        }

        return c;
    }

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.elements.iter()
    }
}

impl<T: Clone> Ring<T> {
    #[allow(dead_code)]
    pub fn as_vec(&self) -> Vec<T> {
        self.iter().cloned().collect()
    }
}

impl<T> ops::Index<usize> for Ring<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl<T> ops::IndexMut<usize> for Ring<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.elements[index]
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
        assert_eq!(r.remove_focused(), Some(3));
        assert_eq!(r.focused(), Some(&2));
        assert_eq!(r.remove_focused(), Some(2));
        assert_eq!(r.focused(), Some(&1));
        assert_eq!(r.remove_focused(), Some(1));
        assert_eq!(r.focused(), None);
        assert_eq!(r.remove_focused(), None);
    }

    #[test]
    fn remove_by() {
        let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        r.focused = 3;
        assert_eq!(r.focused(), Some(&4));
        assert_eq!(r.remove_by(|e| e % 2 == 0), Some(2));
        assert_eq!(r.focused(), Some(&5));
    }

    #[test]
    fn focus_by() {
        let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(r.focus_by(|e| e % 2 == 0), Some(&2));
        assert_eq!(r.focus_by(|e| e % 7 == 0), None);
    }

    #[test]
    fn cycle_focus() {
        let mut r = Ring::new(vec![1, 2, 3]);
        assert_eq!(r.cycle_focus(Direction::Forward), Some(&2));
        assert_eq!(r.as_vec(), vec![1, 2, 3]);
        assert_eq!(r.cycle_focus(Direction::Backward), Some(&1));
        assert_eq!(r.as_vec(), vec![1, 2, 3]);
    }
}
