//! User definable window arangements for a Workspace.
//!
//! Layouts are maintained per monitor and allow for indepent management of the two paramaters
//! (`max_main`, `ratio`) that are used to modify layout logic. Layout functions are only called
//! when there is a need to re-layout a given screen and will always be given a full list of
//! [Clients][1] that the [WindowManager][2] considers tiled. There are no restrictions as to
//! whether or not windows may overlap or that they provide a total covering of the available
//! screen space. Gaps and borders will be added to the [Regions][3] that are specified by layout
//! functions by eating into the regions specified, so there is no need to account for this when
//! writing a layout function.
//!
//! # Writing a simple layout function
//!
//! Lets start with a very basic layout that ignores the two paramaters (`max_main` and `ratio`)
//! and instead, simply arranges the Clients it is given as evenly spaced rows:
//! ```
//! use penrose::core::{
//!     client::Client,
//!     data_types::{Change, Region, ResizeAction},
//!     xconnection::Xid,
//! };
//!
//! pub fn rows(
//!     clients: &[&Client],
//!     _focused: Option<Xid>,
//!     monitor_region: &Region,
//!     _max_main: u32,
//!     _ratio: f32,
//! ) -> Vec<ResizeAction> {
//!     monitor_region
//!         .as_rows(clients.len() as u32)
//!         .iter()
//!         .zip(clients)
//!         .map(|(r, c)| (c.id(), Some(*r)))
//!         .collect()
//! }
//! ```
//!
//! Here we are making use of the [as_rows][4] method on `Region` to split the region we are given
//! (the total available space on the current screen) into evenly sized rows. (There are a number of
//! utility methods on `Region` to aid in writing layout functions.) We then pair each client with
//! `Some(region)` to indicate that this is where the client should be placed by the
//! `WindowManager`. If we provide `None` for any of the clients, that client will then instead be
//! hidden.
//!
//! *Note, windows are positioned and mapped in order, meaning that later clients will overlap
//! those that have already been positioned if any of the Regions overlap one another.*
//!
//! This simple `rows` layout is a sub-set of the behaviour provided by the built in
//! [side_stack][5] layout (in effect, clamping `max_main` at 0).
//!
//! [1]: crate::core::client::Client
//! [2]: crate::core::manager::WindowManager
//! [3]: crate::core::data_types::Region
//! [4]: crate::core::data_types::Region::as_rows
//! [5]: crate::core::layout::side_stack
use crate::{
    common::{
        geometry::{Region, ResizeAction},
        Change, Xid,
    },
    core::client::Client,
};
use std::{cmp, fmt};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// When and how a Layout should be applied.
///
/// The default layout config that only triggers when clients are added / removed and follows user
/// defined config options.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct LayoutConf {
    /// If true, this layout function will not be called to produce resize actions
    pub floating: bool,
    /// Should gaps be dropped regardless of config
    pub gapless: bool,
    /// Should this layout be triggered by window focus as well as add/remove client
    pub follow_focus: bool,
    /// Should cycling clients wrap at the first and last client?
    pub allow_wrapping: bool,
}

impl Default for LayoutConf {
    fn default() -> Self {
        Self {
            floating: false,
            gapless: false,
            follow_focus: false,
            allow_wrapping: true,
        }
    }
}

/// A function that can be used to position Clients on a Workspace.
///
/// Will be called with the current client list, the active client ID (if there is one), the size
/// of the screen that the workspace is shown on and the current values of n_main and ratio for
/// this layout.
pub type LayoutFunc = fn(&[&Client], Option<Xid>, &Region, u32, f32) -> Vec<ResizeAction>;

/// Responsible for arranging Clients within a Workspace.
///
/// A Layout is primarily a function that will be passed an array of Clients to apply resize actions
/// to. Only clients that should be tiled for the current monitor will be passed so no checks are
/// required to see if each client should be handled. The region passed to the layout function
/// represents the current screen dimensions that can be utilised and gaps/borders will be added to
/// each client by the WindowManager itself so there is no need to handle that in the layouts
/// themselves.
///
/// Layouts are expected to have a "main area" that holds the clients with primary focus and any
/// number of secondary areas for the remaining clients to be tiled.
///
/// The user can increase/decrease the size of the main area by setting `ratio` via key bindings
/// which should determine the relative size of the main area compared to other cliens.  Layouts
/// maintain their own state for number of clients in the main area and ratio which will be passed
/// through to the layout function when it is called.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub struct Layout {
    pub(crate) conf: LayoutConf,
    pub(crate) symbol: String,
    max_main: u32,
    ratio: f32,
    #[cfg_attr(feature = "serde", serde(skip))]
    f: Option<LayoutFunc>,
}

impl cmp::PartialEq<Layout> for Layout {
    // Ignoring 'f'
    fn eq(&self, other: &Layout) -> bool {
        self.conf == other.conf
            && self.symbol == other.symbol
            && self.max_main == other.max_main
            && self.ratio == other.ratio
    }
}

impl fmt::Debug for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layout")
            .field("kind", &self.conf)
            .field("symbol", &self.symbol)
            .field("max_main", &self.max_main)
            .field("ratio", &self.ratio)
            .field("f", &stringify!(&self.f))
            .finish()
    }
}

/// A no-op floating layout that simply satisfies the type required for Layout
pub fn floating(_: &[&Client], _: Option<Xid>, _: &Region, _: u32, _: f32) -> Vec<ResizeAction> {
    vec![]
}

impl Layout {
    /// Create a new Layout for a specific monitor
    pub fn new(
        symbol: impl Into<String>,
        conf: LayoutConf,
        f: LayoutFunc,
        max_main: u32,
        ratio: f32,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            conf,
            max_main,
            ratio,
            f: Some(f),
        }
    }

    /// A default floating layout that will not attempt to manage windows
    pub fn floating(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            conf: LayoutConf {
                floating: true,
                gapless: false,
                follow_focus: false,
                allow_wrapping: true,
            },
            f: Some(floating),
            max_main: 1,
            ratio: 1.0,
        }
    }

    // NOTE: Used when rehydrating from serde based deserialization. The layout will panic if
    //       used before setting the LayoutFunc. See [WindowManager::hydrate_and_init]
    #[cfg(feature = "serde")]
    pub(crate) fn set_layout_function(&mut self, f: LayoutFunc) {
        self.f = Some(f);
    }

    /// Apply the layout function held by this `Layout` using the current max_main and ratio
    pub fn arrange(
        &self,
        clients: &[&Client],
        focused: Option<Xid>,
        r: &Region,
    ) -> Vec<ResizeAction> {
        (self.f.expect("missing layout function"))(clients, focused, r, self.max_main, self.ratio)
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
pub(crate) fn mock_layout(
    clients: &[&Client],
    _: Option<Xid>,
    region: &Region,
    _: u32,
    _: f32,
) -> Vec<ResizeAction> {
    clients
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (x, y, w, h) = region.values();
            let _k = i as u32;
            (c.id(), Some(Region::new(x + _k, y + _k, w - _k, h - _k)))
        })
        .collect()
}

/// A simple layout that places the main region on the left and tiles remaining
/// windows in a single column to the right.
pub fn side_stack(
    clients: &[&Client],
    _: Option<Xid>,
    monitor_region: &Region,
    max_main: u32,
    ratio: f32,
) -> Vec<ResizeAction> {
    let n = clients.len() as u32;

    if n <= max_main || max_main == 0 {
        return monitor_region
            .as_rows(n)
            .iter()
            .zip(clients)
            .map(|(r, c)| (c.id(), Some(*r)))
            .collect();
    }

    let split = ((monitor_region.w as f32) * ratio) as u32;
    let (main, stack) = monitor_region.split_at_width(split).unwrap();

    main.as_rows(max_main)
        .into_iter()
        .chain(stack.as_rows(n.saturating_sub(max_main)))
        .zip(clients)
        .map(|(r, c)| (c.id(), Some(r)))
        .collect()
}

/// A simple layout that places the main region at the top of the screen and tiles
/// remaining windows in a single row underneath.
pub fn bottom_stack(
    clients: &[&Client],
    _: Option<Xid>,
    monitor_region: &Region,
    max_main: u32,
    ratio: f32,
) -> Vec<ResizeAction> {
    let n = clients.len() as u32;

    if n <= max_main || max_main == 0 {
        return monitor_region
            .as_columns(n)
            .iter()
            .zip(clients)
            .map(|(r, c)| (c.id(), Some(*r)))
            .collect();
    }

    let split = ((monitor_region.h as f32) * ratio) as u32;
    let (main, stack) = monitor_region.split_at_height(split).unwrap();

    main.as_columns(max_main)
        .into_iter()
        .chain(stack.as_columns(n.saturating_sub(max_main)))
        .zip(clients)
        .map(|(r, c)| (c.id(), Some(r)))
        .collect()
}

/// A simple monolve layout that places uses the maximum available space for the focused client and
/// unmaps all other windows.
pub fn monocle(
    clients: &[&Client],
    focused: Option<Xid>,
    monitor_region: &Region,
    _: u32,
    _: f32,
) -> Vec<ResizeAction> {
    if let Some(fid) = focused {
        let (mx, my, mw, mh) = monitor_region.values();
        clients
            .iter()
            .map(|c| {
                let cid = c.id();
                if cid == fid {
                    (cid, Some(Region::new(mx, my, mw, mh)))
                } else {
                    (cid, None)
                }
            })
            .collect()
    } else {
        Vec::new()
    }
}
