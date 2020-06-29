use crate::manager::WindowManager;
use crate::monitor::Monitor;
use crate::util::Region;
use std::cmp;

pub trait Layout<'a> {
    fn arrange(&self, mon: &mut Monitor, wm: &'a mut WindowManager<'a>);
    fn name(&self) -> &str; // allows for special case handling of certain layouts
}

pub struct BottomStack {}
impl<'a> Layout<'a> for BottomStack {
    fn name(&self) -> &str {
        "bottom_stack"
    }

    fn arrange(&self, mon: &mut Monitor, wm: &'a mut WindowManager<'a>) {
        let n = mon.n_clients();
        if n == 0 {
            return;
        };

        let n_master = mon.n_master;
        let wr = mon.window_region;

        let (mh, tw, ty) = if n > n_master {
            let _mh = if n_master == 0 {
                0
            } else {
                (mon.master_ratio * wr.h as f32) as usize
            };

            (_mh, wr.w / (n - n_master), wr.y + _mh)
        } else {
            (wr.h, wr.w, wr.y)
        };

        let mut tx = wr.x;
        let mut mx = 0;

        mon.client_list = mon
            .client_list
            .iter_mut()
            .filter(|c| !c.is_floating)
            .enumerate()
            .map(|(i, client)| {
                if i < n_master {
                    let w = (wr.w - mx) as usize / cmp::min(n, n_master - i);
                    wm.resize(
                        Region {
                            x: wr.x + mx,
                            y: wr.y,
                            w: w - (2 * client.base_width),
                            h: mh - (2 * client.base_width),
                        },
                        client,
                        false,
                    );
                    mx += client.width();
                } else {
                    wm.resize(
                        Region {
                            x: tx,
                            y: ty,
                            w: tw - (2 * client.base_width),
                            h: wr.h - mh - (2 * client.base_width),
                        },
                        client,
                        false,
                    );
                    if tw != wr.w {
                        tx += client.width();
                    };
                }
                return client.clone();
            })
            .collect();
    }
}
