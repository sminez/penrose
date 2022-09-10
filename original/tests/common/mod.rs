use penrose::{
    common::{
        bindings::{KeyBindings, KeyCode, KeyEventHandler},
        geometry::Region,
    },
    core::{
        layout::*, manager::WindowManager, ring::Selector, screen::Screen, workspace::Workspace,
    },
    xconnection::XConn,
    Forward,
};

use std::collections::HashMap;

const SCREEN_WIDTH: u32 = 1000;
const SCREEN_HEIGHT: u32 = 600;
pub const EXIT_CODE: KeyCode = KeyCode { mask: 0, code: 0 };
pub const LAYOUT_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 1 };
pub const WORKSPACE_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 2 };
pub const SCREEN_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 3 };
pub const FOCUS_CHANGE_CODE: KeyCode = KeyCode { mask: 0, code: 4 };
pub const KILL_CLIENT_CODE: KeyCode = KeyCode { mask: 0, code: 5 };
pub const ADD_WORKSPACE_CODE: KeyCode = KeyCode { mask: 0, code: 6 };
pub const CLIENT_TO_WORKSPACE_CODE: KeyCode = KeyCode { mask: 0, code: 7 };

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

fn layouts() -> Vec<Layout> {
    vec![Layout::new("t", LayoutConf::default(), side_stack, 1, 0.6)]
}

pub fn test_bindings<X: XConn>() -> KeyBindings<X> {
    let mut bindings = HashMap::new();
    bindings.insert(
        EXIT_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.exit()) as KeyEventHandler<X>,
    );
    bindings.insert(
        LAYOUT_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.cycle_layout(Forward)) as KeyEventHandler<X>,
    );
    bindings.insert(
        WORKSPACE_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.focus_workspace(&Selector::Index(1)))
            as KeyEventHandler<X>,
    );
    bindings.insert(
        ADD_WORKSPACE_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.push_workspace(Workspace::new("new", layouts())))
            as KeyEventHandler<X>,
    );
    bindings.insert(
        SCREEN_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.cycle_screen(Forward)) as KeyEventHandler<X>,
    );
    bindings.insert(
        FOCUS_CHANGE_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.cycle_client(Forward)) as KeyEventHandler<X>,
    );
    bindings.insert(
        KILL_CLIENT_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.kill_client()) as KeyEventHandler<X>,
    );
    bindings.insert(
        CLIENT_TO_WORKSPACE_CODE,
        Box::new(|wm: &mut WindowManager<X>| wm.client_to_workspace(&Selector::Index(1)))
            as KeyEventHandler<X>,
    );

    bindings
}
