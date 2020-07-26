//! Information on connected displays
use crate::data_types::{Point, Region};
use xcb;
use xcb::base::Reply;
use xcb::ffi::randr::xcb_randr_get_crtc_info_reply_t;

type CRTCInfoReply = Reply<xcb_randr_get_crtc_info_reply_t>;

/// Display information for a connected screen
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Screen {
    /// The dimensions of the screen
    pub true_region: Region,
    /// The dimensions of the screen if bar is showing
    pub effective_region: Region,
    /// The current workspace index being displayed
    pub wix: usize,
}

impl Screen {
    /// Create a new Screen from information obtained from the X server
    pub fn from_crtc_info_reply(r: CRTCInfoReply, wix: usize) -> Screen {
        let region = Region::new(
            r.x() as u32,
            r.y() as u32,
            r.width() as u32,
            r.height() as u32,
        );

        Screen {
            true_region: region,
            effective_region: region,
            wix,
        }
    }

    pub fn update_effective_region(&mut self, bar_height: u32, top_bar: bool) {
        let (x, y, w, h) = self.true_region.values();
        self.effective_region = if top_bar {
            Region::new(x, y + bar_height, w, h - bar_height)
        } else {
            Region::new(x, y, w, h - bar_height)
        }
    }

    pub fn region(&self, effective_only: bool) -> Region {
        if effective_only {
            self.effective_region
        } else {
            self.true_region
        }
    }

    pub fn contains(&self, p: Point) -> bool {
        let (x1, y1, w, h) = self.true_region.values();
        let (x2, y2) = (x1 + w, x1 + h);

        return p.x >= x1 && p.x < x2 && p.y >= y1 && p.y < y2;
    }
}
