use crate::manager::WindowManager;
use std::collections::HashMap;

pub type LayoutFunc = Box<dyn Fn(usize, &Region, usize, f32) -> Vec<Region>>;
pub type FireAndForget = Box<dyn Fn(&mut WindowManager) -> ()>;
pub type KeyBindings = HashMap<KeyCode, FireAndForget>;
pub type CodeMap = HashMap<String, u8>;

#[derive(Clone, Copy, Debug)]
pub struct Region {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
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
