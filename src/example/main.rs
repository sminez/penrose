/**
 * penrose :: A tiling window manager in the style of dwm
 *
 * Simple configuration can be done by modifying the contents of config.rs,
 * for anything not covered there you should be able to edit the source
 * code with minimal difficulty.
 * To follow the start-up logic and main loop, start with manager.rs
 *
 * The C-level xcb API documentation can be found in the following places:
 *   online manpages: https://www.mankier.com/package/libxcb-devel
 *   official:        https://xcb.freedesktop.org/XcbApi/
 */
#[macro_use]
extern crate penrose;

use penrose::layout::{floating, side_stack};
use penrose::{ColorScheme, Config, Layout, LayoutKind, WindowManager};

fn main() {
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
        Layout::new("[    ]", LayoutKind::Floating, floating, n_main, ratio),
    ];

    let terminal = "st";
    let key_bindings = gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!("rofi-apps"),
        "M-Return" => run_external!(terminal),

        // client management
        "M-j" => run_internal!(next_client),
        "M-k" => run_internal!(previous_client),
        "M-S-q" => run_internal!(kill_client),

        // Layout & window management
        "M-A-Up" => run_internal!(inc_main),
        "M-A-Down" => run_internal!(dec_main),
        "M-A-Right" => run_internal!(inc_ratio),
        "M-A-Left" => run_internal!(dec_ratio),
        "M-A-Escape" => run_internal!(exit);

        forall_workspaces: workspaces => {
            "M-{}" => switch_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    let floating_classes = &["rofi", "dmenu", "dunst"];

    let mut wm = WindowManager::init(Config {
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
        respect_resize_hints: true,
    });

    wm.grab_keys_and_run(key_bindings);
}
