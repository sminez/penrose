//! Simple data types and enums
use crate::layout::Layout;
use crate::manager::WindowManager;
use std::collections::HashMap;
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
#[derive(Debug, Copy, Clone)]
pub enum Direction {
    /// increase the index, wrapping if needed
    Forward,
    /// decrease the index, wrapping if needed
    Backward,
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
pub struct Ring<T> {
    elements: Vec<T>,
    focused: usize,
}

impl<T> Ring<T> {
    pub fn new(elements: Vec<T>) -> Ring<T> {
        Ring {
            elements,
            focused: 0,
        }
    }

    /**
     * Take a reference to the currently focused element if there is one
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r1 = Ring::new(vec![1, 2, 3]);
     * assert_eq!(r1.focused(), Some(&1));
     *
     * let mut r2: Ring<()> = Ring::new(vec![]);
     * assert_eq!(r2.focused(), None);
     * ```
     */
    pub fn focused(&self) -> Option<&T> {
        if self.elements.len() > 0 {
            Some(&self.elements[self.focused])
        } else {
            None
        }
    }

    /**
     * Take a mutable reference to the currently focused element if there is one
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r1 = Ring::new(vec![1, 2, 3]);
     * assert_eq!(r1.focused_mut(), Some(&mut 1));
     *
     * let mut r2: Ring<()> = Ring::new(vec![]);
     * assert_eq!(r2.focused_mut(), None);
     * ```
     */
    pub fn focused_mut(&mut self) -> Option<&mut T> {
        if self.elements.len() > 0 {
            Some(&mut self.elements[self.focused])
        } else {
            None
        }
    }

    /**
     * Rotate the elements of the Ring but maintain focus at the current index
     * ```
     * use penrose::data_types::{Direction, Ring};
     *
     * let mut r = Ring::new(vec![1, 2, 3]);
     * r.rotate(Direction::Forward);
     * assert_eq!(r.as_vec(), &vec![3, 1, 2]);
     * assert_eq!(r.focused(), Some(&3));
     * r.rotate(Direction::Backward);
     * assert_eq!(r.as_vec(), &vec![1, 2, 3]);
     * assert_eq!(r.focused(), Some(&1));
     * ```
     */
    pub fn rotate(&mut self, direction: Direction) {
        if self.elements.len() > 1 {
            match direction {
                Direction::Forward => {
                    let last = self.elements.pop().unwrap();
                    self.elements.insert(0, last);
                }
                Direction::Backward => {
                    let first = self.elements.remove(0);
                    self.elements.push(first);
                }
            }
        }
    }

    /**
     * Move the focus point of the ring forward / backward through the elements
     * but leave their order unchanged.
     * ```
     * use penrose::data_types::{Direction, Ring};
     *
     * let mut r = Ring::new(vec![1, 2, 3]);
     * assert_eq!(r.cycle_focus(Direction::Forward), Some(&2));
     * assert_eq!(r.as_vec(), &vec![1, 2, 3]);
     * assert_eq!(r.cycle_focus(Direction::Backward), Some(&1));
     * assert_eq!(r.as_vec(), &vec![1, 2, 3]);
     * ```
     */
    pub fn cycle_focus(&mut self, direction: Direction) -> Option<&T> {
        let max = self.elements.len() - 1;
        self.focused = match direction {
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
        };
        self.focused()
    }

    /**
     * Move the focused element forward/backward through the ring while
     * retaining focus.
     * ```
     * use penrose::data_types::{Direction, Ring};
     *
     * let mut r = Ring::new(vec![1, 2, 3]);
     * assert_eq!(r.focused(), Some(&1));
     * assert_eq!(r.drag_focused(Direction::Forward), Some(&1));
     * assert_eq!(r.as_vec(), &vec![3, 1, 2]);
     * assert_eq!(r.drag_focused(Direction::Forward), Some(&1));
     * assert_eq!(r.as_vec(), &vec![2, 3, 1]);
     * ```
     */
    pub fn drag_focused(&mut self, direction: Direction) -> Option<&T> {
        self.rotate(direction);
        self.cycle_focus(direction)
    }

    /**
     * Set the currently focused index and return a reference to the focused element
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec!["this", "that", "other"]);
     * assert_eq!(r.set_focus(2), &"other");
     * ```
     */
    pub fn set_focus(&mut self, index: usize) -> &T {
        self.focused = index;
        &self.elements[self.focused]
    }

    /**
     * Focus the first element satisfying the given condition returning Some(&T) if an
     * element was located, otherwise None.
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
     * assert_eq!(r.focus_by(|e| e % 2 == 0), Some(&2));
     * assert_eq!(r.focus_by(|e| e % 7 == 0), None);
     * ```
     */
    pub fn focus_by(&mut self, cond: impl Fn(&T) -> bool) -> Option<&T> {
        if let Some((i, _)) = self.elements.iter().enumerate().find(|(_, e)| cond(*e)) {
            self.focused = i;
            Some(&self.elements[self.focused])
        } else {
            None
        }
    }

    /**
     * Return the length of the underlying Vec<T>
     * ```
     * use penrose::data_types::Ring;
     *
     * let r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
     * assert_eq!(r.len(), 6);
     * ```
     */
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /**
     * Add a new element at 'index', leaving the focused index unchanged.
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
     * assert_eq!(r.set_focus(3), &4);
     * r.insert(2, 42);
     * assert_eq!(r.focused(), Some(&3));
     * ```
     */
    pub fn insert(&mut self, index: usize, element: T) {
        self.elements.insert(index, element);
    }

    /**
     * Remove the first element satisfying the given condition, maintaining the
     * current focus position if possible.
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec![1, 2, 3, 4, 5, 6]);
     * assert_eq!(r.set_focus(3), &4);
     * assert_eq!(r.remove_by(|e| e % 2 == 0), Some(2));
     * assert_eq!(r.focused(), Some(&5));
     * ```
     */
    pub fn remove_by(&mut self, cond: impl Fn(&T) -> bool) -> Option<T> {
        if let Some((i, _)) = self.elements.iter().enumerate().find(|(_, e)| cond(*e)) {
            if self.focused > 0 && self.focused == self.elements.len() - 1 {
                self.focused -= 1;
            }
            Some(self.elements.remove(i))
        } else {
            None
        }
    }

    /**
     * If this Ring has at least one element, remove the focused element
     * and return it, otherwise None
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec![1, 2, 3]);
     * assert_eq!(r.set_focus(2), &3);
     * assert_eq!(r.remove_focused(), Some(3));
     * assert_eq!(r.focused(), Some(&2));
     * assert_eq!(r.remove_focused(), Some(2));
     * assert_eq!(r.focused(), Some(&1));
     * assert_eq!(r.remove_focused(), Some(1));
     * assert_eq!(r.focused(), None);
     * assert_eq!(r.remove_focused(), None);
     * ```
     */
    pub fn remove_focused(&mut self) -> Option<T> {
        if self.elements.len() == 0 {
            return None;
        }

        let c = self.elements.remove(self.focused);
        // correct the focus point if we are now out of bounds
        if self.focused > 0 && self.focused >= self.elements.len() - 1 {
            self.focused -= 1;
        }

        Some(c)
    }

    /**
     * If ix is within bounds, remove that element and return it, otherwise None
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec!['a', 'b', 'c']);
     * assert_eq!(r.remove_at(1), Some('b'));
     * assert_eq!(r.remove_at(3), None);
     * ```
     */
    pub fn remove_at(&mut self, ix: usize) -> Option<T> {
        let max = self.elements.len() - 1;
        if ix <= max {
            if self.focused == max {
                self.focused -= 1;
            }
            Some(self.elements.remove(ix))
        } else {
            None
        }
    }

    /**
     * A reference to the underlying Vec<T> wrapped by this Ring
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec!['a', 'b', 'c']);
     * assert_eq!(r.as_vec(), &vec!['a', 'b', 'c']);
     * ```
     */
    pub fn as_vec(&self) -> &Vec<T> {
        &self.elements
    }

    /**
     * Iterate over the elements of this ring in their current order
     * ```
     * use penrose::data_types::Ring;
     *
     * let r = Ring::new(vec![1, 2, 3, 4]);
     * assert_eq!(r.iter().map(|c| c + 1).collect::<Vec<i32>>(), vec![2, 3, 4, 5]);
     * ```
     */
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.elements.iter()
    }

    /**
     * Mutably iterate over the elements of this ring in their current order
     * ```
     * use penrose::data_types::Ring;
     *
     * let mut r = Ring::new(vec![1, 2, 3]);
     * r.iter_mut().for_each(|e| *e *= 2);
     * r.set_focus(2);
     * assert_eq!(r.focused(), Some(&6));
     * ```
     */
    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.elements.iter_mut()
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
        assert_eq!(r.focused(), Some(&3));
        r.rotate(Direction::Backward);
        assert_eq!(r.focused(), Some(&1));
    }
}
