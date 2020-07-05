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
    pub fn current_outputs(conn: &mut xcb::Connection) -> Vec<Screen> {
        let screen = match conn.get_setup().roots().nth(0) {
            None => die!("unable to get handle for screen"),
            Some(s) => s,
        };

        let win_id = conn.generate_id();
        let root = screen.root();

        // xcb docs: https://www.mankier.com/3/xcb_create_window
        xcb::create_window(
            conn,   // xcb connection to X11
            0,      // new window's depth
            win_id, // ID to be used for referring to the window
            root,   // parent window
            0,      // x-coordinate
            0,      // y-coordinate
            1,      // width
            1,      // height
            0,      // border width
            0,      // class (i _think_ 0 == COPY_FROM_PARENT?)
            0,      // visual (i _think_ 0 == COPY_FROM_PARENT?)
            &[],    // value list? (value mask? not documented either way...)
        );

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_screen_resources
        let resources = xcb::randr::get_screen_resources(conn, win_id);

        // xcb docs: https://www.mankier.com/3/xcb_randr_get_crtc_info
        return match resources.get_reply() {
            Err(e) => die!("error reading X screen resources: {}", e),
            Ok(reply) => reply
                .crtcs()
                .iter()
                .flat_map(|c| xcb::randr::get_crtc_info(conn, *c, 0).get_reply())
                .enumerate()
                .map(|(i, r)| Screen::from_crtc_info_reply(r, i))
                .filter(|r| r.width() > 0)
                .collect(),
        };
    }

    pub fn from_crtc_info_reply(r: CRTCInfoReply, wix: usize) -> Screen {
        let region = Region::new(
            r.x() as usize,
            r.y() as usize,
            r.width() as usize,
            r.height() as usize,
        );

        Screen { region, wix }
    }

    pub fn width(&self) -> usize {
        self.region.width()
    }

    pub fn height(&self) -> usize {
        self.region.height()
    }
}
