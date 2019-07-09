//! Clients are the base level 'windows' being managed.

struct Position {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

pub struct Client {
    name: String,
    tags: u8,
    next: &Client,
    snext: &Client,
    monitor: &Monitor,
    window: &Window,

    position: Position,
    old_position: Position,

    min_alpha: f32,
    max_alpha: f32,

    base_width: i32,
    max_width: i32,
    min_width: i32,
    inc_width: i32,

    base_height: i32,
    max_height: i32,
    min_height: i32,
    inc_height: i32,

    border_width: i32,
    old_border_width: i32,

    is_fixed: bool,
    is_floating: bool,
    is_urgent: bool,
    never_focus: bool,
    old_state: bool,
    is_fullscreen: bool,
    is_pinned: bool,
}
