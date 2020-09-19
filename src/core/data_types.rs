//! Simple data types and enums
use crate::{
    hooks,
    layout::{side_stack, Layout, LayoutConf},
};

/// Output of a Layout function: the new position a window should take
pub type ResizeAction = (WinId, Option<Region>);

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

    /// Create a range from 1 -> n_workspaces for use in keybindings
    pub fn ws_range(&self) -> std::ops::Range<usize> {
        1..(self.workspaces.len() + 1)
    }
}

/* Argument enums */

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
