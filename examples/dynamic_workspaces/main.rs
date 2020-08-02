#[macro_use]
extern crate penrose;

#[macro_use]
extern crate log;

use penrose::core::helpers::spawn_for_output;
use penrose::core::{Hook, Layout, Selector, WindowManager, Workspace, XcbConnection};
use penrose::layout::{bottom_stack, side_stack, LayoutConf};
use penrose::{Backward, Config, Forward, Less, More};

use penrose::contrib::extensions::Scratchpad;
use penrose::contrib::hooks::{DefaultWorkspace, LayoutSymbolAsRootName};
use penrose::contrib::layouts::paper;

use simplelog::{LevelFilter, SimpleLogger};
use std::env;

const WORKSPACES: &[&str] = &["1term", "2term", "3term", "web", "files"];
struct RemoveEmptyWorkspaces {}
impl Hook for RemoveEmptyWorkspaces {
    fn workspace_change(&mut self, wm: &mut WindowManager, old: usize, _: usize) {
        let sel = Selector::Index(old);
        if let Some(ws) = wm.workspace(&sel) {
            info!("ws name: {}", ws.name());
            if WORKSPACES.contains(&ws.name()) && ws.len() == 0 {
                wm.remove_workspace(&sel);
            }
        };
    }
}

fn my_layouts() -> Vec<Layout> {
    let n_main = 1;
    let ratio = 0.6;
    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
    };

    vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
    ]
}

fn create_or_switch_to_workspace(wm: &mut WindowManager) {
    let script = format!("{}/bin/ws_spawn.sh", env::var("HOME").unwrap());
    let output = spawn_for_output(script);
    let ws_name = output.trim_end();

    let cond = |ws: &Workspace| ws.name() == ws_name;
    let sel = Selector::Condition(&cond);

    if wm.workspace(&sel).is_none() {
        wm.push_workspace(Workspace::new(ws_name, my_layouts()))
    }
    wm.focus_workspace(&sel);
}

fn main() {
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default()).unwrap();
    let mut config = Config::default();
    config.workspaces = vec!["main"];
    config.layouts = my_layouts();
    config.hooks = vec![
        LayoutSymbolAsRootName::new(),
        Box::new(RemoveEmptyWorkspaces {}),
        DefaultWorkspace::new("1term", "[side]", vec!["st"]),
        DefaultWorkspace::new("2term", "[botm]", vec!["st", "st"]),
        DefaultWorkspace::new("3term", "[side]", vec!["st", "st", "st"]),
        DefaultWorkspace::new("web", "[papr]", vec!["firefox"]),
        DefaultWorkspace::new("files", "[botm]", vec!["thunar"]),
    ];

    let sp = Scratchpad::new("st", 0.8, 0.8);
    sp.register(&mut config);

    let key_bindings = gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!("rofi-apps"),
        "M-Return" => run_external!("st"),

        // client management
        "M-j" => run_internal!(cycle_client, Forward),
        "M-k" => run_internal!(cycle_client, Backward),
        "M-S-j" => run_internal!(drag_client, Forward),
        "M-S-k" => run_internal!(drag_client, Backward),
        "M-S-q" => run_internal!(kill_client),
        "M-slash" => sp.toggle(),

        // workspace management
        "M-w" => Box::new(create_or_switch_to_workspace),
        "M-Tab" => run_internal!(toggle_workspace),
        "M-bracketright" => run_internal!(cycle_screen, Forward),
        "M-bracketleft" => run_internal!(cycle_screen, Backward),
        "M-S-bracketright" => run_internal!(drag_workspace, Forward),
        "M-S-bracketleft" => run_internal!(drag_workspace, Backward),

        // Layout management
        "M-grave" => run_internal!(cycle_layout, Forward),
        "M-S-grave" => run_internal!(cycle_layout, Backward),
        "M-A-Up" => run_internal!(update_max_main, More),
        "M-A-Down" => run_internal!(update_max_main, Less),
        "M-A-Right" => run_internal!(update_main_ratio, More),
        "M-A-Left" => run_internal!(update_main_ratio, Less),

        "M-A-s" => run_internal!(detect_screens),

        "M-A-Escape" => run_internal!(exit);

        // setting up bindings for 9 workspaces
        forall_workspaces: &[1,2,3,4,5,6,7,8,9] => {
            "M-{}" => focus_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    let conn = XcbConnection::new();
    let mut wm = WindowManager::init(config, &conn);
    wm.grab_keys_and_run(key_bindings);
}
