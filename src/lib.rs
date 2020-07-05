#[macro_use]
pub mod macros;

pub mod client;
pub mod config;
pub mod helpers;
pub mod layout;
pub mod manager;
pub mod screen;
pub mod workspace;

pub mod data_types {
    use crate::manager::WindowManager;
    use std::collections::HashMap;
    use xcb;

    pub type FireAndForget = Box<dyn Fn(&mut WindowManager) -> ()>;
    pub type KeyBindings = HashMap<KeyCode, FireAndForget>;
    pub type ResizeAction = (WinId, Region);
    pub type CodeMap = HashMap<String, u8>;
    pub type WinId = u32;

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

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct ColorScheme {
        pub bg: &'static str,
        pub fg_1: &'static str,
        pub fg_2: &'static str,
        pub fg_3: &'static str,
        pub hl: &'static str,
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
