//! Simple data types and enums
use crate::{
    hooks,
    layout::{side_stack, Layout, LayoutConf},
    manager::WindowManager,
};

use std::{
    collections::{HashMap, VecDeque},
    ops,
};

use xcb;

/// Some action to be run by a user key binding
pub type FireAndForget = Box<dyn FnMut(&mut WindowManager) -> ()>;

/// User defined key bindings
pub type KeyBindings = HashMap<KeyCode, FireAndForget>;

/// Output of a Layout function: the new position a window should take
pub type ResizeAction = (WinId, Option<Region>);

/// Map xmodmap key names to their X key code so that we can bind them by name
pub type CodeMap = HashMap<String, u8>;

/// An X window ID
pub type WinId = u32;

/// An x,y coordinate pair
#[derive(Debug, Copy, Clone)]
pub struct Point {
    /// An absolute x coordinate relative to the root window
    pub x: u32,
    /// An absolute y coordinate relative to the root window
    pub y: u32,
}

impl Point {
    /// Create a new Point.
    pub fn new(x: u32, y: u32) -> Point {
        Point { x, y }
    }
}

/// The main user facing configuration details
pub struct Config<'a> {
    /// Default workspace names to use when initialising the WindowManager. Must have at least one element.
    pub workspaces: Vec<&'a str>,
    /// WM_CLASS values that should always be treated as floating.
    pub floating_classes: &'static [&'static str],
    /// Default Layouts to be given to every workspace.
    pub layouts: Vec<Layout>,
    /// Focused boder color
    pub focused_border: u32,
    /// Unfocused boder color
    pub unfocused_border: u32,
    /// The width of window borders in pixels
    pub border_px: u32,
    /// The size of gaps between windows in pixels.
    pub gap_px: u32,
    /// The percentage change in main_ratio to be applied when increasing / decreasing.
    pub main_ratio_step: f32,
    /// Whether or not space should be reserved for a status bar
    pub show_bar: bool,
    /// True if the status bar should be at the top of the screen, false if it should be at the bottom
    pub top_bar: bool,
    /// Height of space reserved for status bars in pixels
    pub bar_height: u32,
    /// User supplied Hooks for modifying WindowManager behaviour
    pub hooks: Vec<Box<dyn hooks::Hook>>,
}

impl<'a> Config<'a> {
    /// Initialise a default Config, giving sensible (but minimal) values for all fields.
    pub fn default() -> Config<'a> {
        Config {
            workspaces: vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"],
            floating_classes: &["dmenu", "dunst"],
            layouts: vec![
                Layout::new("[side]", LayoutConf::default(), side_stack, 1, 0.6),
                Layout::floating("[----]"),
            ],
            focused_border: 0xcc241d,   // #cc241d
            unfocused_border: 0x3c3836, // #3c3836
            border_px: 2,
            gap_px: 5,
            main_ratio_step: 0.05,
            show_bar: true,
            top_bar: true,
            bar_height: 18,
            hooks: vec![],
        }
    }
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
    /// Invert this Direction
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

/// Where a given element should be inserted into a Ring
#[derive(Debug, Copy, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Region {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

impl Region {
    /// Create a new Region.
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Region {
        Region { x, y, w, h }
    }

    /// Destructure this Region into its component values (x, y, w, h).
    pub fn values(&self) -> (u32, u32, u32, u32) {
        (self.x, self.y, self.w, self.h)
    }
}

/// An X key-code along with a modifier mask
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct KeyCode {
    /// Modifier key bit mask
    pub mask: u16,
    /// X key code
    pub code: u8,
}

impl KeyCode {
    /// Build a new KeyCode from an XCB KeyPressEvent
    pub fn from_key_press(k: &xcb::KeyPressEvent) -> KeyCode {
        KeyCode {
            mask: k.state(),
            code: k.detail(),
        }
    }
}

/// Used with WindowManager helper functions to select an element from the
/// known workspaces or clients.
#[derive(Clone, Copy)]
pub enum Selector<'a, T> {
    /// The focused element of the target collection.
    Focused,
    /// The element at this index.
    Index(usize),
    /// The element with/containing this client ID.
    WinId(WinId),
    /// The first element satisfying this condition.
    Condition(&'a dyn Fn(&T) -> bool),
}

/**
 * A Collection<T> that has both an order for its elements and a focused element
 * at some index.
 *
 * Supports rotating the position of the elements and rotating which element
 * is focused independently of one another.
 */
#[derive(Debug, Clone, PartialEq)]
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

    pub fn focused_index(&self) -> usize {
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

    pub fn iter(&self) -> std::collections::vec_deque::Iter<T> {
        self.elements.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::vec_deque::IterMut<T> {
        self.elements.iter_mut()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.elements.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.elements.get_mut(index)
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

    pub fn index(&self, s: &Selector<T>) -> Option<usize> {
        match s {
            Selector::WinId(_) => None, // ignored
            Selector::Focused => Some(self.focused_index()),
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

    pub fn element(&self, s: &Selector<T>) -> Option<&T> {
        match s {
            Selector::WinId(_) => None, // ignored
            Selector::Focused => self.focused(),
            Selector::Index(i) => self.elements.get(*i),
            Selector::Condition(f) => self.element_by(f).map(|(_, e)| e),
        }
    }

    pub fn element_mut(&mut self, s: &Selector<T>) -> Option<&mut T> {
        match s {
            Selector::Focused => self.focused_mut(),
            Selector::Index(i) => self.elements.get_mut(*i),
            Selector::WinId(_) => None, // ignored
            Selector::Condition(f) => self.element_by_mut(f).map(|(_, e)| e),
        }
    }

    pub fn all_elements(&self, s: &Selector<T>) -> Vec<&T> {
        match s {
            Selector::WinId(_) => vec![], // ignored
            Selector::Focused => self.focused().into_iter().collect(),
            Selector::Index(i) => self.elements.get(*i).into_iter().collect(),
            Selector::Condition(f) => self.elements.iter().filter(|e| f(*e)).collect(),
        }
    }

    pub fn all_elements_mut(&mut self, s: &Selector<T>) -> Vec<&mut T> {
        match s {
            Selector::WinId(_) => vec![], // ignored
            Selector::Focused => self.focused_mut().into_iter().collect(),
            Selector::Index(i) => self.elements.get_mut(*i).into_iter().collect(),
            Selector::Condition(f) => self.elements.iter_mut().filter(|e| f(*e)).collect(),
        }
    }

    pub fn focus(&mut self, s: &Selector<T>) -> Option<&T> {
        match s {
            Selector::WinId(_) => None, // ignored
            Selector::Focused => self.focused(),
            Selector::Index(i) => {
                self.focused = *i;
                self.focused()
            }
            Selector::Condition(f) => {
                if let Some((i, _)) = self.element_by(f) {
                    self.focused = i;
                    Some(&self.elements[self.focused])
                } else {
                    None
                }
            }
        }
    }

    pub fn remove(&mut self, s: &Selector<T>) -> Option<T> {
        match s {
            Selector::WinId(_) => None, // ignored
            Selector::Focused => {
                let c = self.elements.remove(self.focused);
                self.clamp_focus();
                return c;
            }
            Selector::Index(i) => {
                let c = self.elements.remove(*i);
                self.clamp_focus();
                return c;
            }
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
    pub fn equivalent_selectors(&self, s: &Selector<T>, t: &Selector<T>) -> bool {
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
    fn remove_by() {
        let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        r.focused = 3;
        assert_eq!(r.focused(), Some(&4));
        assert_eq!(r.remove(&Selector::Condition(&|e| e % 2 == 0)), Some(2));
        assert_eq!(r.focused(), Some(&5));
    }

    #[test]
    fn focus_by() {
        let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
        assert_eq!(r.focus(&Selector::Condition(&|e| e % 2 == 0)), Some(&2));
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
}
