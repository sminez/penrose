/**
 * Layouts are maintained per monitor and allow for indepent management of the two
 * paramaters (n_main, main_ratio) that are used to modify layout logic. As penrose
 * makes use of a tagging system as opposed to workspaces, layouts will be passed a
 * Vec of clients to handle which is determined by the current client and monitor tags.
 * arrange is only called if there are clients to handle so there is no need to check
 * that clients.len() > 0. r is the monitor Region defining the size of the monitor
 * for the layout to position windows.
 */
use crate::config;
use crate::data_types::{Region, ResizeAction, WinId};

/**
 * Almost all layouts will be 'Normal' but penrose allows both for layouts that
 * explicitly remove gaps and window borders and for floating layouts that do
 * not apply resize actions to their windows.
 * While it is possible to have multiple floating layouts, there isn't much
 * point as kind == Floating disables calling through to the wrapped layout
 * function.
 */
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LayoutKind {
    Floating, // Floating layouts will not apply window resizing
    Gapless,  // Prevent borders and gaps being added to windows
    Normal,   // Gaps and borders will be added as per config.rs
}

/**
 * A Layout is primarily a function that will be passed an array of client IDs
 * to apply resize actions to. Only clients that should be tiled for the current
 * monitor will be passed so no checks are required to see if each client should
 * be handled. The region passed to the layout function represents the current
 * screen dimensions that can be utilised and gaps/borders will be added to
 * each client by the WindowManager itself so there is no need to handle that
 * in the layouts themselves.
 * Layouts are expected to have a 'main area' that holds the clients with primary
 * focus and any number of secondary areas for the remaining clients to be tiled.
 * The user can increase/decrease the size of the main area by setting 'ratio'
 * via key bindings which should determine the relative size of the main area
 * compared to other cliens.
 * Layouts maintain their own state for number of clients in the main area and
 * ratio which will be passed through to the layout function when it is called.
 *
 * NOTE: currently _not_ doing this as a trait in order to avoid painful generic
 *       nonsense...maybe something to tackle later when I'm feeling up to it!
 */
#[derive(Clone, Copy)]
pub struct Layout {
    pub kind: LayoutKind,
    pub symbol: &'static str,
    max_main: usize,
    ratio: f32,
    f: fn(&Vec<WinId>, &Region, usize, f32) -> Vec<ResizeAction>,
}

impl Layout {
    /// Create a new Layout for a specific monitor
    pub fn new(
        symbol: &'static str,
        kind: LayoutKind,
        f: fn(&Vec<WinId>, &Region, usize, f32) -> Vec<ResizeAction>,
    ) -> Layout {
        Layout {
            symbol,
            kind,
            max_main: config::MAX_MAIN,
            ratio: config::MAIN_RATIO,
            f,
        }
    }

    /// Apply the embedded layout function using the current n_main and ratio
    pub fn arrange(&self, client_ids: &Vec<WinId>, r: &Region) -> Vec<ResizeAction> {
        (self.f)(client_ids, r, self.max_main, self.ratio)
    }

    /// Increase/decrease the number of clients in the main area by 1
    pub fn update_max_main(&mut self, increase: bool) {
        if increase {
            self.max_main += 1;
        } else {
            if self.max_main > 0 {
                self.max_main -= 1;
            }
        }
    }

    /// Increase/decrease the size of the main area relative to secondary.
    /// (clamps at 1.0 and 0.0 respectively)
    pub fn update_main_ratio(&mut self, increase: bool) {
        if increase {
            self.ratio += config::MAIN_RATIO_STEP;
        } else {
            self.ratio -= config::MAIN_RATIO_STEP;
        }
        if self.ratio < 0.0 {
            self.ratio = 0.0
        } else if self.ratio > 1.0 {
            self.ratio = 1.0;
        }
    }
}

/*
 * Utility functions for simplifying writing layouts
 *
 * NOTE: public so that they can be used by layouts in other files if needed.
 */

/// number of clients for the main area vs secondary
pub fn client_breakdown(clients: &Vec<WinId>, n_main: usize) -> (usize, usize) {
    let n = clients.len();
    if n <= n_main {
        (n, 0)
    } else {
        (n_main, n - n_main)
    }
}

/*
 * Layout functions
 *
 * Each of the following is a layout function that can be passed to Layout::new.
 * No checks are carried out to ensure that clients are tiled correctly (i.e. that
 * they are non-overlapping) so when adding additional layout functions you are
 * free to tile them however you wish. Xmonad for example has a 'circle' layout
 * that deliberately overlaps clients under the main window.
 */

/// A no-op floating layout that simply satisfies the type required for Layout
pub fn floating(
    _clients: &Vec<WinId>,
    _monitor_region: &Region,
    _max_main: usize,
    _ratio: f32,
) -> Vec<ResizeAction> {
    vec![]
}

/// A simple layout that places the main region on the left and tiles remaining
/// windows in a single column to the right.
pub fn side_stack(
    client_ids: &Vec<WinId>,
    monitor_region: &Region,
    max_main: usize,
    ratio: f32,
) -> Vec<ResizeAction> {
    let (_, _, mw, mh) = monitor_region.values();
    let (n_main, n_stack) = client_breakdown(&client_ids, max_main);
    let h_stack = if n_stack > 0 { mh / n_stack } else { 0 };
    let h_main = if n_main > 0 { mh / n_main } else { 0 };
    let split = if max_main > 0 {
        (mw as f32 * ratio) as usize
    } else {
        0
    };

    client_ids
        .iter()
        .enumerate()
        .map(|(n, id)| {
            if n < max_main {
                let w = if n_stack == 0 { mw } else { split };
                (*id, Region::new(0, n * h_main, w, h_main))
            } else {
                let sn = n - max_main; // nth stacked client
                (*id, Region::new(split, sn * h_stack, mw - split, h_stack))
            }
        })
        .collect()
}
