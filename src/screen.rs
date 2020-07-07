use crate::data_types::Region;
use xcb;
use xcb::base::Reply;
use xcb::ffi::randr::xcb_randr_get_crtc_info_reply_t;

type CRTCInfoReply = Reply<xcb_randr_get_crtc_info_reply_t>;

pub struct Screen {
    pub region: Region, // screen dimensions
    pub wix: usize,     // active workspace
}

impl Screen {
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
