//! Additional layout functions
use crate::core::{
    client::Client,
    data_types::{Region, ResizeAction},
    xconnection::Xid,
};

/// A layout that aims to mimic the feel of having multiple pieces of paper fanned out on a desk.
///
/// Without access to the custom hardware required for 10gui, we instead have to rely on the WM
/// actions we have available: n_main is ignored and instead, the focused client takes up ratio% of
/// the screen, with the remaining windows being stacked on top of one another to either side. Think
/// fanning out a hand of cards and then pulling one out and placing it on top of the fan.
pub fn paper(
    clients: &[&Client],
    focused: Option<Xid>,
    monitor_region: &Region,
    _: u32,
    ratio: f32,
) -> Vec<ResizeAction> {
    let n = clients.len();
    if n == 1 {
        return vec![(clients[0].id(), Some(*monitor_region))];
    }

    let (mx, my, mw, mh) = monitor_region.values();
    let min_w = 0.5; // clamp client width at 50% screen size (we're effectively fancy monocle)
    let cw = (mw as f32 * if ratio > min_w { ratio } else { min_w }) as u32;
    let step = (mw - cw) / (n - 1) as u32;

    let fid = focused.unwrap(); // we know we have at least one client now
    let mut after_focused = false;
    clients
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let cid = c.id();
            if cid == fid {
                after_focused = true;
                (cid, Some(Region::new(mx + i as u32 * step, my, cw, mh)))
            } else {
                let mut x = mx + i as u32 * step;
                if after_focused {
                    x += cw - step
                };
                (cid, Some(Region::new(x, my, step, mh)))
            }
        })
        .collect()
}

fn dwindle_recurisive(
    clients: &[&Client],
    region: &Region,
    horizontal: bool,
    min_size: u32,
) -> Vec<ResizeAction> {
    if clients.len() > 1 {
        if region.w < min_size || region.h < min_size {
            clients
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    if i == 0 {
                        (c.id(), Some(*region))
                    } else {
                        (c.id(), None)
                    }
                })
                .collect()
        } else {
            let split = ((if horizontal { region.w } else { region.h } as f32) / 2.) as u32;
            let (main, other) = if horizontal {
                region.split_at_width(split)
            } else {
                region.split_at_height(split)
            }
            .unwrap();

            let mut vec =
                dwindle_recurisive(&clients[..clients.len() - 1], &other, !horizontal, min_size);
            vec.push((clients.last().unwrap().id(), Some(main)));
            vec
        }
    } else {
        clients
            .get(0)
            .map(|c| vec![(c.id(), Some(*region))])
            .unwrap_or(Vec::new())
    }
}

/// A layout based on the dwindle layout from AwesomeWM.
///
/// The second region is recursively split in two other regions, alternating between
/// splitting horizontally and vertically.
pub fn dwindle(
    clients: &[&Client],
    _: Option<Xid>,
    monitor_region: &Region,
    _: u32,
    _: f32,
) -> Vec<ResizeAction> {
    dwindle_recurisive(clients, monitor_region, true, 50)
}
