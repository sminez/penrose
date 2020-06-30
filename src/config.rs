//! Config and settings for your penrose build
//! While it is encouraged to edit / extend the source in order to customise Penrose,
//! there are a number of built in settings that can be modified by simply editing the
//! variables in this file.

pub const BORDER_PX: usize = 2; // border size in pixels for windows
pub const GAP_PX: usize = 6; // gap size in pixels between windows (0 disables gaps, fullscreen has no gap)
pub const SNAP: usize = 12; // snap pixel

pub const SYSTRAY_SPACING: usize = 2; // spacing between icons in the systray
pub const SYSTRAY_PINNING: usize = 0; // 0 -> systray follows active monitor, n -> pin systray to monitor n
pub const SYSTRAY_PINNING_FAIL_FIRST: bool = true; // if pinning fails: true -> display systray on the first monitor, false -> display it on the last
pub const SHOW_SYSTRAY: bool = true; // should the systray be shown by Penrose? (set to false if you are launching an independed systray)

pub const SHOW_BAR: bool = true; // should the status bar be shown? (set to false if using something like polybar/lemonbar)
pub const TOP_BAR: bool = true; // true -> status bar renders at the top of the screen, false -> it renders at the bottom

// color scheme
pub const COLOR_GRAY_1: &str = "#282828";
pub const COLOR_GRAY_2: &str = "#3c3836";
pub const COLOR_GRAY_3: &str = "#a89984";
pub const COLOR_GRAY_4: &str = "#f2e5bc";
pub const COLOR_HIGHLIGHT: &str = "#458588";

// fonts to use for rendering UI
pub const FONTS: [&str; 2] = ["ProFont For Powerline:size=10", "Iosevka Nerd Font:size=10"];

// names for each of the tags
pub const TAGS: [&str; 9] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

// path to an executable that will be run on initial WM start-up
pub const STARTUP_SCRIPT_PATH: Option<&str> = Some("~/bin/scripts/start-dwm.sh");

// layout config
pub const MAIN_RATIO: f32 = 0.60; // factor of main area size [0.05..0.95]
pub const N_MAIN: usize = 1; // number of clients in main area
pub const RESPECT_RESIZE_HINTS: bool = true; // true means respect size hints in tiled resizals
