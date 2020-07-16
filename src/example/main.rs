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
use penrose::layout::{bottom_stack, side_stack};
use penrose::{ColorScheme, Config, Layout, LayoutKind, WindowManager, XcbConnection};
use simplelog;
use std::env;

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
    let color_scheme = ColorScheme {
        bg: 0x282828,        // #282828
        fg_1: 0x3c3836,      // #3c3836
        fg_2: 0xa89984,      // #a89984
        fg_3: 0xf2e5bc,      // #f2e5bc
        highlight: 0xcc241d, // #cc241d
        urgent: 0x458588,    // #458588
    };

    let n_main = 1;
    let ratio = 0.6;
    let layouts = vec![
        Layout::new("[side]", LayoutKind::Normal, side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutKind::Normal, bottom_stack, n_main, ratio),
        // Layout::new("[    ]", LayoutKind::Floating, floating, n_main, ratio),
    ];

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
        "M-j" => run_internal!(next_client),
        "M-k" => run_internal!(previous_client),
        "M-S-j" => run_internal!(drag_client_forward),
        "M-S-k" => run_internal!(drag_client_backward),
        "M-S-q" => run_internal!(kill_client),

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace),

        // Layout & window management
        "M-grave" => run_internal!(next_layout),
        "M-S-grave" => run_internal!(previous_layout),
        "M-A-Up" => run_internal!(inc_main),
        "M-A-Down" => run_internal!(dec_main),
        "M-A-Right" => run_internal!(inc_ratio),
        "M-A-Left" => run_internal!(dec_ratio),
        "M-A-Escape" => run_internal!(exit);

        forall_workspaces: workspaces => {
            "M-{}" => focus_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    let floating_classes = &["rofi", "dmenu", "dunst", "polybar"];

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
    wm.grab_keys_and_run(key_bindings);
}
