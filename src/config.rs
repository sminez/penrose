use crate::data_types::ColorScheme;
use crate::manager::KeyBindings;

pub const FONTS: &[&str] = &["ProFont For Powerline:size=10", "Iosevka Nerd Font:size=10"];
pub const TAGS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];
pub const STARTUP_SCRIPT: &str = "~/bin/scripts/start-dwm.sh";
pub const COLOR_SCHEME: ColorScheme = ColorScheme {
    bg: "#282828",
    fg_1: "#3c3836",
    fg_2: "#a89984",
    fg_3: "#f2e5bc",
    hl: "#458588",
};
pub const BORDER_PX: usize = 2;
pub const GAP_PX: usize = 6;
pub const SYSTRAY_SPACING: usize = 2;
pub const SHOW_SYSTRAY: bool = true;
pub const SHOW_BAR: bool = true;
pub const TOP_BAR: bool = true;
pub const MAIN_RATIO: f32 = 0.60;
pub const N_MAIN: usize = 1;
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
        "M-semicolon" => run_external!("rofi-apps"),
        "M-A-r" => run_internal!(kill);

        forall_tags: TAGS => {
            "M-{}" => set_tag,
            "M-C-{}" => add_tag,
            "M-S-{}" => tag_client,
        }
    }
}
