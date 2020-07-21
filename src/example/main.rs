/**
 * penrose :: A tiling window manager in the style of dwm
 *
 * To follow the start-up logic and main loop, start with manager.rs
 *
 * The C-level xcb API documentation can be found in the following places:
 *   online manpages: https://www.mankier.com/package/libxcb-devel
 *   official:        https://xcb.freedesktop.org/XcbApi/
 */
#[macro_use]
extern crate penrose;

use penrose::helpers::spawn;
use penrose::layout::{bottom_stack, paper, side_stack};
use penrose::{
    Backward, ColorScheme, Config, Forward, Layout, LayoutConf, Less, More, WindowManager,
    XcbConnection,
};
use simplelog;
use std::env;
use std::process::Command;

fn main() {
    // Turn on debug logging for non-release builds
    simplelog::SimpleLogger::init(
        if cfg!(debug_assertions) {
            simplelog::LevelFilter::Debug
        } else {
            simplelog::LevelFilter::Info
        },
        simplelog::Config::default(),
    )
    .unwrap();

    let workspaces = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
    let fonts = &["ProFont For Powerline:size=10", "Iosevka Nerd Font:size=10"];
    let floating_classes = &["rofi", "dmenu", "dunst", "polybar", "pinentry-gtk-2"];
    let color_scheme = ColorScheme {
        bg: 0x282828,        // #282828
        fg_1: 0x3c3836,      // #3c3836
        fg_2: 0xa89984,      // #a89984
        fg_3: 0xf2e5bc,      // #f2e5bc
        highlight: 0xcc241d, // #cc241d
        urgent: 0x458588,    // #458588
    };

    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
    };
    let n_main = 1;
    let ratio = 0.6;
    let layouts = vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
        Layout::floating("[----]"),
    ];

    // I run penrose wrapped in a shell script that redirects the log output to a file and allows
    // me to restart without killing the session. "real" exit is done via 'pkill x'
    let power_menu = Box::new(move |wm: &mut WindowManager| {
        let choice = Command::new(format!(
            "{}/bin/scripts/power-menu.sh",
            env::var("HOME").unwrap()
        ))
        .output()
        .unwrap();
        match String::from_utf8(choice.stdout).unwrap().as_str() {
            "restart-wm\n" => wm.exit(),
            _ => (), // 'no', user exited out or something went wrong
        }
        None
    });

    // Set the root X window name to be the active layout symbol so it can be picked up by polybar
    let active_layout_as_root_name = |wm: &mut WindowManager| {
        wm.set_root_window_name(wm.current_layout_symbol());
    };

    let browser = "qutebrowser";
    let terminal = "st";

    let key_bindings = gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!("rofi-apps"),
        "M-b" => run_external!(browser),
        "M-Return" => run_external!(terminal),

        // actions
        "M-A-s" => run_external!("screenshot"),
        "M-A-k" => run_external!("toggle-kb-for-tada"),
        "M-A-l" => run_external!("lock-screen"),
        "M-A-m" => run_external!("xrandr --output HDMI-1 --auto --right-of eDP-1 "),

        // client management
        "M-j" => run_internal!(cycle_client, Forward),
        "M-k" => run_internal!(cycle_client, Backward),
        "M-S-j" => run_internal!(drag_client, Forward),
        "M-S-k" => run_internal!(drag_client, Backward),
        "M-S-q" => run_internal!(kill_client),

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace),
        "M-bracketright" => run_internal!(cycle_screen, Forward),
        "M-bracketleft" => run_internal!(cycle_screen, Backward),

        // Layout & window management
        "M-grave" => Box::new(move |wm: &mut WindowManager| {
            wm.cycle_layout(Forward);
            active_layout_as_root_name(wm);
            None
        }),
        "M-S-grave" => Box::new(move |wm: &mut WindowManager| {
            wm.cycle_layout(Backward);
            active_layout_as_root_name(wm);
            None
        }),
        "M-A-Up" => run_internal!(update_max_main, More),
        "M-A-Down" => run_internal!(update_max_main, Less),
        "M-A-Right" => run_internal!(update_main_ratio, More),
        "M-A-Left" => run_internal!(update_main_ratio, Less),
        "M-A-C-Escape" => run_internal!(exit),
        "M-A-Escape" => power_menu;

        forall_workspaces: workspaces => {
            "M-{}" => focus_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    let conn = XcbConnection::new();

    let mut wm = WindowManager::init(
        Config {
            workspaces: workspaces,
            fonts: fonts,
            floating_classes: floating_classes,
            layouts: layouts,
            color_scheme: color_scheme,
            border_px: 2,
            gap_px: 5,
            main_ratio_step: 0.05,
            systray_spacing_px: 2,
            show_systray: true,
            show_bar: true,
            top_bar: true,
            bar_height: 18,
            respect_resize_hints: true,
        },
        &conn,
    );

    spawn(format!(
        "{}/bin/scripts/penrose-startup.sh",
        env::var("HOME").unwrap()
    ));

    active_layout_as_root_name(&mut wm);
    wm.grab_keys_and_run(key_bindings);
}
