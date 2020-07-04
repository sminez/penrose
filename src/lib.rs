#[macro_use]
pub mod macros;

pub mod client;
pub mod config;
pub mod helpers;
pub mod layout;
pub mod manager;

pub mod data_types {
    use crate::manager::WindowManager;
    use std::collections::HashMap;
    use xcb;

    pub type LayoutFunc = Box<dyn Fn(usize, &Region, usize, f32) -> Vec<Region>>;
    pub type FireAndForget = Box<dyn Fn(&mut WindowManager) -> ()>;
    pub type KeyBindings = HashMap<KeyCode, FireAndForget>;
    pub type ResizeAction = (WinId, Option<Region>);
    pub type CodeMap = HashMap<String, u8>;
    pub type WinId = i32;

    type CRTCInfoReply = xcb::ffi::randr::xcb_randr_get_crtc_info_reply_t;

    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct Region {
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    }

    impl Region {
        pub fn from_crtc_info_reply(r: xcb::base::Reply<CRTCInfoReply>) -> Region {
            Region {
                x: r.x() as u32,
                y: r.y() as u32,
                w: r.width() as u32,
                h: r.height() as u32,
            }
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
