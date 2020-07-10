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
#[derive(Debug)]
pub enum Direction {
    /// increase the index, wrapping if needed
    Forward,
    /// decrease the index, wrapping if needed
    Backward,
}

/// Increment / decrement a value
#[derive(Debug)]
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

#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    pub bg: u32,
    pub fg_1: u32,
    pub fg_2: u32,
    pub fg_3: u32,
    pub highlight: u32,
    pub urgent: u32,
}

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
 * A Collection<T> that has both the notion of an order of elements and a
 * focused element at some index. Supports rotating the position of the
 * elements and rotating which element is focused independently of one another.
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

    pub fn focused(&self) -> Option<&T> {
        if self.elements.len() > 0 {
            Some(&self.elements[self.focused])
        } else {
            None
        }
    }

    pub fn focused_mut(&mut self) -> Option<&mut T> {
        if self.elements.len() > 0 {
            Some(&mut self.elements[self.focused])
        } else {
            None
        }
    }

    pub fn rotate(&mut self, direction: Direction) {
        if self.elements.len() > 1 {
            match direction {
                Direction::Forward => {
                    let first = self.elements.remove(0);
                    self.elements.push(first);
                }
                Direction::Backward => {
                    let last = self.elements.pop().unwrap();
                    self.elements.insert(0, last);
                }
            }
        }
    }

    pub fn cycle_focus(&mut self, direction: Direction) {
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
    }

    pub fn set_focus(&mut self, index: usize) {
        self.focused = index;
    }

    /// Focus the first element satisfying the given condition returning true if an
    /// element was located, false otherwise.
    pub fn focus_by(&mut self, cond: impl Fn(&T) -> bool) -> bool {
        if let Some((i, _)) = self.elements.iter().enumerate().find(|(_, e)| cond(*e)) {
            self.focused = i;
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.elements.insert(index, element);
    }

    /// Remove the first element satisfying the given condition, maintaining the
    /// current focus position if possible.
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

    /// If this Ring has at least one element, remove the focused element
    /// and return it, otherwise None
    pub fn remove_focused(&mut self) -> Option<T> {
        if self.elements.len() > 0 {
            let c = self.elements.remove(self.focused);
            // correct the focus point if we are now out of bounds
            if self.focused > 0 && self.focused == self.elements.len() - 1 {
                self.focused -= 1;
            }
            Some(c)
        } else {
            None
        }
    }

    /// If ix is within bounds, remove that element and return it, otherwise None
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

    /// A reference to the underlying Vec<T> wrapped by this Ring
    pub fn as_vec(&self) -> &Vec<T> {
        &self.elements
    }

    /// Iterate over the elements of this ring in their current order
    pub fn iter(&self) -> std::slice::Iter<T> {
        self.elements.iter()
    }

    /// Mutably iterate over the elements of this ring in their current order
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
