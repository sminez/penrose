//! Information on connected displays
use crate::core::data_types::{Point, Region};

/// Display information for a connected screen
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Screen {
    /// The current workspace index being displayed
    pub wix: usize,
    true_region: Region,
    effective_region: Region,
}

impl PartialOrd for Screen {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.true_region.x.partial_cmp(&other.true_region.x)
    }
}

impl Screen {
    /// Create a new screen instance directly
    pub fn new(region: Region, wix: usize) -> Screen {
        Screen {
            true_region: region,
            effective_region: region,
            wix,
        }
    }

    /// Cache the current effective region of this screen based on whether or not a bar is
    /// displayed and if that bar is positioned at the top or bottom of the screen.
    pub fn update_effective_region(&mut self, bar_height: u32, top_bar: bool) {
        let (x, y, w, h) = self.true_region.values();
        self.effective_region = if top_bar {
            Region::new(x, y + bar_height, w, h - bar_height)
        } else {
            Region::new(x, y, w, h - bar_height)
        }
    }

    /// The available space for displaying clients on this screen. If 'effective_only' then the
    /// returned Region will account for space taken up by a bar.
    pub fn region(&self, effective_only: bool) -> Region {
        if effective_only {
            self.effective_region
        } else {
            self.true_region
        }
    }

    /// Determine whether or not an absolute coordinate Point (relative to the root window) is
    /// located on this screen.
    pub fn contains(&self, p: Point) -> bool {
        self.true_region.contains_point(&p)
    }
}
