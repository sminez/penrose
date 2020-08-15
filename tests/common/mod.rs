use penrose::data_types::{FireAndForget, KeyBindings, KeyCode, Region, Selector};
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
pub const KILL_CLIENT_CODE: KeyCode = KeyCode { mask: 0, code: 5 };

pub fn simple_screen(n: usize) -> Screen {
    Screen::new(
        Region::new(
            n as u32 * SCREEN_WIDTH,
            n as u32 * SCREEN_HEIGHT,
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
        ),
        n,
    )
}

pub fn test_bindings() -> KeyBindings {
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
        Box::new(|wm: &mut WindowManager| wm.focus_workspace(&Selector::Index(1))) as FireAndForget,
    );
    bindings.insert(
        SCREEN_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager| wm.cycle_screen(Forward)) as FireAndForget,
    );
    bindings.insert(
        FOCUS_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager| wm.cycle_client(Forward)) as FireAndForget,
    );
    bindings.insert(
        KILL_CLIENT_CODE,
        Box::new(|wm: &mut WindowManager| wm.kill_client()) as FireAndForget,
    );

    bindings
}
