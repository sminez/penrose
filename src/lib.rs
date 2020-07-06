#[macro_use]
pub mod macros;

pub mod client;
pub mod helpers;
pub mod layout;
pub mod manager;
pub mod screen;
pub mod workspace;

// top level re-exports
pub use data_types::{ColorScheme, Config};
pub use layout::{Layout, LayoutKind};
pub use manager::WindowManager;

pub mod data_types {
    use crate::layout::Layout;
    use crate::manager::WindowManager;
    use std::collections::HashMap;
    use xcb;

    pub type FireAndForget = Box<dyn Fn(&mut WindowManager) -> ()>;
    pub type KeyBindings = HashMap<KeyCode, FireAndForget>;
    pub type ResizeAction = (WinId, Region);
    pub type CodeMap = HashMap<String, u8>;
    pub type WinId = u32;

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
        pub respect_resize_hints: bool,
    }

    /*
     * Argument enums
     */

    pub enum Direction {
        Forward,
        Backward,
    }

    pub enum Change {
        More,
        Less,
    }

    pub enum Border {
        Urgent,
        Focused,
        Unfocused,
    }

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Region {
        x: usize,
        y: usize,
        w: usize,
        h: usize,
    }

    impl Region {
        pub fn new(x: usize, y: usize, w: usize, h: usize) -> Region {
            Region { x, y, w, h }
        }

        pub fn width(&self) -> usize {
            self.w
        }

        pub fn height(&self) -> usize {
            self.h
        }

        pub fn values(&self) -> (usize, usize, usize, usize) {
            (self.x, self.y, self.w, self.h)
        }
    }

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
}
