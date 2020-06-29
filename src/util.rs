use crate::config;
use std::process;

// A rectangular region on a screen. Specified by top left corner and width / height
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Region {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Region {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Region {
        Region { x, y, w, h }
    }
}

// Run the user startup script if there is one defined
pub fn run_autostart() {
    if let Some(path) = config::STARTUP_SCRIPT_PATH {
        process::Command::new(path)
            .spawn()
            .expect("failed to spwan startup script");
    }
}

/*
 * NOTE: The penrose tagging system
 * penrose follows the dwm model of applying tags to clients and allowing
 * the user to determine which tags are shown on which monitors, as opposed
 * to assigning clients to workspaces and determining a single workspace to
 * display on each monitor. This allows for multiple tags to be displayed at
 * once and for client pinning in a simple way.
 * To enable bit masking as our method of checking / setting tags, tag indices
 * start from 1 not 0.
 * Currently, tag toggles are checked and kill the process if they are invalid
 * in order to simplify finding bugs in config (this may change at a later date).
 */

pub fn toggle_tag(current: usize, tag_index: usize) -> usize {
    if tag_index > config::TAGS.len() {
        eprintln!("{} is an invalid tag", tag_index);
        process::exit(1);
    }

    current & (1 << tag_index)
}
