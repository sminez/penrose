/// A rectangular region on a screen. Specified by top left corner and width / height
#[derive(Debug, PartialEq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
