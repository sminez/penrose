/*
 * NOTE: Penrose layouts
 * layouts are maintained per monitor and allow for indepent management of the two
 * paramaters (n_main, main_ratio) that are used to modify layout logic. As penrose
 * makes use of a tagging system as opposed to workspaces, layouts will be passed a
 * Vec of clients to handle which is determined by the current client and monitor tags.
 * arrange is only called if there are clients to handle so there is no need to check
 * that clients.len() > 0. wr is the monitor Region defining the size of the monitor
 * for the layout to position windows.
 */
use crate::client::Client;
use crate::util::Region;
use std::cmp;

pub const BOTTOM_STACK: &'static str = "bottom_stack";
pub const FLOATING: &'static str = "floating";
pub const MONOCLE: &'static str = "monocle";

pub trait Layout {
    fn name(&self) -> &str;
    fn set_n_main(&mut self, n: usize);
    fn set_main_ratio(&mut self, r: f32);
    fn arrange(&self, clients: &Vec<&Client>, wr: &Region) -> Vec<Region>;
}

pub struct BottomStack {
    n_main: usize,
    main_ratio: f32,
}

impl Layout for BottomStack {
    fn name(&self) -> &str {
        BOTTOM_STACK
    }

    fn set_n_main(&mut self, n: usize) {
        self.n_main = n;
    }

    fn set_main_ratio(&mut self, r: f32) {
        self.main_ratio = r;
    }

    fn arrange(&self, clients: &Vec<&Client>, wr: &Region) -> Vec<Region> {
        let n = clients.len();
        let mut tx = wr.x;
        let mut mx = 0;

        let (mh, tw, ty) = if n > self.n_main {
            let _mh = if self.n_main == 0 {
                0
            } else {
                (self.main_ratio * wr.h as f32) as usize
            };

            (_mh, wr.w / (n - self.n_main), wr.y + _mh)
        } else {
            (wr.h, wr.w, wr.y)
        };

        clients
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let bw = c.base_width;
                if i < self.n_main {
                    let r = Region {
                        x: wr.x + mx,
                        y: wr.y,
                        w: ((wr.w - mx) as usize / cmp::min(n, self.n_main - i)) - (2 * bw),
                        h: mh - (2 * bw),
                    };
                    mx += c.width_on_resize(r);

                    r
                } else {
                    let r = Region {
                        x: tx,
                        y: ty,
                        w: tw - (2 * bw),
                        h: wr.h - mh - (2 * bw),
                    };
                    if tw != wr.w {
                        tx += c.width_on_resize(r);
                    };

                    r
                }
            })
            .collect()
    }
}
