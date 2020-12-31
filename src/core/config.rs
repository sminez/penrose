//! User facing configuration of the penrose [WindowManager][crate::core::manager::WindowManager].
use crate::core::layout::{side_stack, Layout, LayoutConf};

use std::fmt;

/// The main user facing configuration details
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, PartialEq)]
pub struct Config {
    /// Default workspace names to use when initialising the WindowManager. Must have at least one element.
    pub workspaces: Vec<String>,
    /// WM_CLASS values that should always be treated as floating.
    pub floating_classes: Vec<String>,
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
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("workspaces", &self.workspaces)
            .field("floating_classes", &self.floating_classes)
            .field("layouts", &self.layouts)
            .field("focused_border", &self.focused_border)
            .field("unfocused_border", &self.unfocused_border)
            .field("border_px", &self.border_px)
            .field("gap_px", &self.gap_px)
            .field("main_ratio_step", &self.main_ratio_step)
            .field("show_bar", &self.show_bar)
            .field("top_bar", &self.top_bar)
            .field("bar_height", &self.bar_height)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            workspaces: ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            floating_classes: ["dmenu", "dunst"].iter().map(|s| s.to_string()).collect(),
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
        }
    }
}

impl Config {
    /// Create a range from 1 -> n_workspaces for use in keybindings
    pub fn ws_range(&self) -> std::ops::Range<usize> {
        1..(self.workspaces.len() + 1)
    }

    /// Set the workspaces field on this Config
    pub fn workspaces(&mut self, val: Vec<impl Into<String>>) -> &mut Self {
        self.workspaces = val.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set the floating_classes field on this Config
    pub fn floating_classes(&mut self, val: Vec<impl Into<String>>) -> &mut Self {
        self.floating_classes = val.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set the layouts field on this Config
    pub fn layouts(&mut self, val: Vec<Layout>) -> &mut Self {
        self.layouts = val;
        self
    }

    /// Set the focused_border field on this Config
    pub fn focused_border(&mut self, val: u32) -> &mut Self {
        self.focused_border = val;
        self
    }

    /// Set the unfocused_border field on this Config
    pub fn unfocused_border(&mut self, val: u32) -> &mut Self {
        self.unfocused_border = val;
        self
    }

    /// Set the border_px field on this Config
    pub fn border_px(&mut self, val: u32) -> &mut Self {
        self.border_px = val;
        self
    }

    /// Set the gap_px field on this Config
    pub fn gap_px(&mut self, val: u32) -> &mut Self {
        self.gap_px = val;
        self
    }

    /// Set the main_ratio_step field on this Config
    pub fn main_ratio_step(&mut self, val: f32) -> &mut Self {
        self.main_ratio_step = val;
        self
    }

    /// Set the show_bar field on this Config
    pub fn show_bar(&mut self, val: bool) -> &mut Self {
        self.show_bar = val;
        self
    }

    /// Set the top_bar field on this Config
    pub fn top_bar(&mut self, val: bool) -> &mut Self {
        self.top_bar = val;
        self
    }

    /// Set the bar_height field on this Config
    pub fn bar_height(&mut self, val: u32) -> &mut Self {
        self.bar_height = val;
        self
    }
}
