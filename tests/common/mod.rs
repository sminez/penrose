use penrose::data_types::{FireAndForget, KeyBindings, KeyCode, Region};
use penrose::screen::Screen;
use penrose::{Forward, WindowManager};
use std::collections::HashMap;

const SCREEN_WIDTH: u32 = 1000;
const SCREEN_HEIGHT: u32 = 600;
pub const EXIT_CODE: KeyCode = KeyCode { mask: 0, code: 0 };
pub const LAYOUT_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 1 };
pub const WORKSPACE_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 2 };
pub const SCREEN_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 3 };
pub const FOCUS_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 4 };

pub fn simple_screen(n: u32) -> Screen {
    let r = Region::new(
        n * SCREEN_WIDTH,
        n * SCREEN_HEIGHT,
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
    );

    Screen {
        true_region: r,
        effective_region: r,
        wix: n as usize,
    }
}

pub fn exit_bindings() -> KeyBindings {
    let mut bindings = HashMap::new();
    bindings.insert(
        EXIT_CODE,
        Box::new(|wm: &mut WindowManager| wm.exit()) as FireAndForget,
    );
    bindings.insert(
        LAYOUT_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager| wm.cycle_layout(Forward)) as FireAndForget,
    );
    bindings.insert(
        WORKSPACE_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager| wm.focus_workspace(1)) as FireAndForget,
    );
    bindings.insert(
        SCREEN_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager| wm.cycle_screen(Forward)) as FireAndForget,
    );
    bindings.insert(
        FOCUS_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager| wm.cycle_client(Forward)) as FireAndForget,
    );

    bindings
}
