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
use crate::config;
use crate::util::Region;
use std::cmp;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LayoutKind {
    Floating,
    Gapless,
    Normal,
}

pub fn layouts() -> Vec<Layout> {
    vec![Layout::new(LayoutKind::Normal, bottom_stack)]
}

pub struct ResizeAction {
    pub c: Client,
    pub r: Option<Region>,
}

#[derive(Clone)]
pub struct Layout {
    pub kind: LayoutKind,
    n_main: usize,
    ratio: f32,
    f: fn(Vec<Client>, &Region, usize, f32) -> Vec<ResizeAction>,
}

impl Layout {
    fn new(
        kind: LayoutKind,
        f: fn(Vec<Client>, &Region, usize, f32) -> Vec<ResizeAction>,
    ) -> Layout {
        Layout {
            kind,
            n_main: config::N_MAIN,
            ratio: config::MAIN_RATIO,
            f,
        }
    }

    pub fn arrange(&self, cs: Vec<Client>, r: &Region) -> Vec<ResizeAction> {
        (self.f)(cs, r, self.n_main, self.ratio)
    }

    pub fn set_n_main(&mut self, n: usize) {
        self.n_main = n;
    }

    pub fn set_main_ratio(&mut self, r: f32) {
        self.ratio = r;
    }
}

/*
 * Layout functions
 */

fn bottom_stack(clients: Vec<Client>, wr: &Region, n_main: usize, ratio: f32) -> Vec<ResizeAction> {
    let n = clients.len();
    let mut tx = wr.x;
    let mut mx = 0;

    let (mh, tw, ty) = if n > n_main {
        let _mh = if n_main == 0 {
            0
        } else {
            (ratio * wr.h as f32) as usize
        };

        (_mh, wr.w / (n - n_main), wr.y + _mh)
    } else {
        (wr.h, wr.w, wr.y)
    };

    clients
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let bw = c.base_width;
            if i < n_main {
                let r = Region {
                    x: wr.x + mx,
                    y: wr.y,
                    w: ((wr.w - mx) as usize / cmp::min(n, n_main - i)) - (2 * bw),
                    h: mh - (2 * bw),
                };
                mx += c.width_on_resize(r);

                ResizeAction { c: *c, r: Some(r) }
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

                ResizeAction { c: *c, r: Some(r) }
            }
        })
        .collect()
}
