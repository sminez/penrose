#[macro_use]
extern crate penrose;

use penrose::core::helpers::spawn_for_output;
use penrose::core::{Layout, WindowManager, XcbConnection};
use penrose::layout::{bottom_stack, side_stack, LayoutConf};
use penrose::{Backward, Config, Forward, Less, More};

use penrose::contrib::actions::create_or_switch_to_workspace;
use penrose::contrib::extensions::Scratchpad;
use penrose::contrib::hooks::{DefaultWorkspace, LayoutSymbolAsRootName, RemoveEmptyWorkspaces};
use penrose::contrib::layouts::paper;

use simplelog::{LevelFilter, SimpleLogger};
use std::env;

fn my_layouts() -> Vec<Layout> {
    let n_main = 1;
    let ratio = 0.6;
    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
        allow_wrapping: false,
    };

    vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
    ]
}

fn main() {
    SimpleLogger::init(LevelFilter::Debug, simplelog::Config::default()).unwrap();
    let mut config = Config::default();
    config.workspaces = vec!["main"];
    config.layouts = my_layouts();
    config.hooks = vec![
        LayoutSymbolAsRootName::new(),
        RemoveEmptyWorkspaces::new(config.workspaces.clone()),
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
        "M-semicolon" => run_external!("dmenu_run"),
        "M-Return" => run_external!("st"),

        // client management
        "M-j" => run_internal!(cycle_client, Forward),
        "M-k" => run_internal!(cycle_client, Backward),
        "M-S-j" => run_internal!(drag_client, Forward),
        "M-S-k" => run_internal!(drag_client, Backward),
        "M-S-q" => run_internal!(kill_client),
        "M-slash" => sp.toggle(),

        // workspace management
        "M-w" => create_or_switch_to_workspace(
            || {
                let output = spawn_for_output(
                    format!("{}/bin/ws_spawn.sh", env::var("HOME").unwrap())
                ).unwrap();
                output.trim_end().to_string()
            },
            my_layouts()
        ),
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

        // setting up bindings for 6 possible workspaces
        forall_workspaces: &[1, 2, 3, 4, 5, 6] => {
            "M-{}" => focus_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    let conn = XcbConnection::new().unwrap();
    let mut wm = WindowManager::init(config, &conn);
    wm.grab_keys_and_run(key_bindings);
}
