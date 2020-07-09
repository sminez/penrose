//! Information on connected displays
use crate::data_types::Region;
use xcb;
use xcb::base::Reply;
use xcb::ffi::randr::xcb_randr_get_crtc_info_reply_t;

type CRTCInfoReply = Reply<xcb_randr_get_crtc_info_reply_t>;

/// Display information for a connected screen
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Screen {
    /// The dimensions of the screen
    pub region: Region,
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

        Screen { region, wix }
    }
}
