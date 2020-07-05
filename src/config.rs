use crate::data_types::{ColorScheme, KeyBindings};
use crate::layout::{floating, side_stack, Layout, LayoutKind};

pub const DEBUG: bool = true;
pub const FONTS: &[&str] = &["ProFont For Powerline:size=10", "Iosevka Nerd Font:size=10"];
pub const WORKSPACES: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
pub const STARTUP_SCRIPT: &str = "~/bin/scripts/start-dwm.sh";
pub const COLOR_SCHEME: ColorScheme = ColorScheme {
    bg: 0x282828,        // #282828
    fg_1: 0x3c3836,      // #3c3836
    fg_2: 0xa89984,      // #a89984
    fg_3: 0xf2e5bc,      // #f2e5bc
    highlight: 0xcc241d, // #cc241d
    urgent: 0x458588,    // #458588
};

pub const SYSTRAY_SPACING: u32 = 2;
pub const SHOW_SYSTRAY: bool = true;
pub const SHOW_BAR: bool = true;
pub const TOP_BAR: bool = true;

pub const BORDER_PX: u32 = 2;
pub const GAP_PX: u32 = 5;

pub const MAX_MAIN: usize = 1;
pub const MAIN_RATIO: f32 = 0.60;
pub const MAIN_RATIO_STEP: f32 = 0.05;

pub const FLOATING_CLASSES: &[&'static str] = &["rofi", "dmenu", "dunst"];
pub const RESPECT_RESIZE_HINTS: bool = true;

/**
 * The strings used in gen_keybindings are parsed into modifier combinations
 * when the WindowManager struct is initialised as follows:
 *   M -> Mod4
 *   A -> Alt
 *   C -> Control
 *   S -> Shift
 *
 * All key names should match those outputted when running 'xmodmap -pke'
 */
pub fn key_bindings() -> KeyBindings {
    gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!("rofi-apps"),
        "M-Return" => run_external!("st"),

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

        forall_workspaces: WORKSPACES => {
            "M-{}" => switch_workspace,
            "M-S-{}" => client_to_workspace,
        }
    }
}

/**
 * This function will be called per monitor to initialise the layouts present
 * on each.
 */
pub fn layouts() -> Vec<Layout> {
    vec![
        Layout::new("[side]", LayoutKind::Normal, side_stack),
        Layout::new("[    ]", LayoutKind::Floating, floating),
    ]
}
