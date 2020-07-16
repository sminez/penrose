/*!
 * Layouts are user definable window arangements for a Workspace.
 *
 * Layouts are maintained per monitor and allow for indepent management of the two
 * paramaters (n_main, main_ratio) that are used to modify layout logic. As penrose
 * makes use of a tagging system as opposed to workspaces, layouts will be passed a
 * Vec of clients to handle which is determined by the current client and monitor tags.
 * arrange is only called if there are clients to handle so there is no need to check
 * that clients.len() > 0. r is the monitor Region defining the size of the monitor
 * for the layout to position windows.
 */
use crate::data_types::{Change, Region, ResizeAction, WinId};
use std::fmt;

/**
 * How a given layout should treat borders, gaps and floating windows.
 *
 * Almost all layouts will be 'Normal' but penrose allows both for layouts that
 * explicitly remove gaps and window borders and for floating layouts that do
 * not apply resize actions to their windows.
 * While it is possible to have multiple floating layouts, there isn't much
 * point as kind == Floating disables calling through to the wrapped layout
 * function.
 */
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LayoutKind {
    /// Floating layouts will not apply window resizing
    Floating,
    /// Prevent borders and gaps being added to windows
    Gapless,
    /// Gaps and borders will be added as per config.rs
    Normal,
}

/**
 * Responsible for arranging windows within a Workspace.
 *
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
 */
#[derive(Clone, Copy)]
pub struct Layout {
    /// How this layout should be applied by the WindowManager
    pub kind: LayoutKind,
    /// User defined symbol for displaying in the status bar
    pub symbol: &'static str,
    max_main: u32,
    ratio: f32,
    f: fn(&[WinId], &Region, u32, f32) -> Vec<ResizeAction>,
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layout")
            .field("kind", &self.kind)
            .field("symbol", &self.symbol)
            .field("max_main", &self.max_main)
            .field("ratio", &self.ratio)
            .field("f", &stringify!(&self.f))
            .finish()
    }
}

impl Layout {
    /// Create a new Layout for a specific monitor
    pub fn new(
        symbol: &'static str,
        kind: LayoutKind,
        f: fn(&[WinId], &Region, u32, f32) -> Vec<ResizeAction>,
        max_main: u32,
        ratio: f32,
    ) -> Layout {
        Layout {
            symbol,
            kind,
            max_main,
            ratio,
            f,
        }
    }

    /// Apply the embedded layout function using the current n_main and ratio
    pub fn arrange(&self, clients: &[WinId], r: &Region) -> Vec<ResizeAction> {
        (self.f)(clients, r, self.max_main, self.ratio)
    }

    /// Increase/decrease the number of clients in the main area by 1
    pub fn update_max_main(&mut self, change: Change) {
        match change {
            Change::More => self.max_main += 1,
            Change::Less => {
                if self.max_main > 0 {
                    self.max_main -= 1;
                }
            }
        }
    }

    /// Increase/decrease the size of the main area relative to secondary.
    /// (clamps at 1.0 and 0.0 respectively)
    pub fn update_main_ratio(&mut self, change: Change, step: f32) {
        match change {
            Change::More => self.ratio += step,
            Change::Less => self.ratio -= step,
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
 */

/// number of clients for the main area vs secondary
pub fn client_breakdown<T>(clients: &[T], n_main: u32) -> (u32, u32) {
    let n = clients.len() as u32;
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

// ignore paramas and return pairs of window ID and index in the client vec
#[cfg(test)]
pub(crate) fn mock_layout(clients: &[WinId], r: &Region, _: u32, _: f32) -> Vec<ResizeAction> {
    clients
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (x, y, w, h) = r.values();
            let k = i as u32;
            (*c, Region::new(x + k, y + k, w - k, h - k))
        })
        .collect()
}

/// A no-op floating layout that simply satisfies the type required for Layout
pub fn floating(_: &[WinId], _: &Region, _: u32, _: f32) -> Vec<ResizeAction> {
    vec![]
}

/**
 * A simple layout that places the main region on the left and tiles remaining
 * windows in a single column to the right.
 */
pub fn side_stack(
    clients: &[WinId],
    monitor_region: &Region,
    max_main: u32,
    ratio: f32,
) -> Vec<ResizeAction> {
    let (mx, my, mw, mh) = monitor_region.values();
    let (n_main, n_stack) = client_breakdown(&clients, max_main);
    let h_stack = if n_stack > 0 { mh / n_stack } else { 0 };
    let h_main = if n_main > 0 { mh / n_main } else { 0 };
    let split = if max_main > 0 {
        (mw as f32 * ratio) as u32
    } else {
        0
    };

    clients
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let n = n as u32;
            if n < max_main {
                let w = if n_stack == 0 { mw } else { split };
                (*c, Region::new(mx, my + n * h_main, w, h_main))
            } else {
                let sn = n - max_main; // nth stacked client
                let region = Region::new(mx + split, my + sn * h_stack, mw - split, h_stack);
                (*c, region)
            }
        })
        .collect()
}

/**
 * A simple layout that places the main region at the top of the screen and tiles
 * remaining windows in a single row underneath.
 */
pub fn bottom_stack(
    clients: &[WinId],
    monitor_region: &Region,
    max_main: u32,
    ratio: f32,
) -> Vec<ResizeAction> {
    let (mx, my, mw, mh) = monitor_region.values();
    let (n_main, n_stack) = client_breakdown(&clients, max_main);
    let split = if max_main > 0 {
        (mh as f32 * ratio) as u32
    } else {
        0
    };
    let h_main = if n_stack > 0 { split } else { mh } / n_main;
    let w_stack = if n_stack > 0 { mw / n_stack } else { 0 };

    clients
        .iter()
        .enumerate()
        .map(|(n, c)| {
            let n = n as u32;
            if n < max_main {
                (*c, Region::new(mx, my + n * h_main, mw, h_main))
            } else {
                let sn = n - max_main; // nth stacked client
                let region = Region::new(mx + sn * w_stack, my + split, w_stack, mh - split);
                (*c, region)
            }
        })
        .collect()
}
