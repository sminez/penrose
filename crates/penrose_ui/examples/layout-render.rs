use penrose::{
    builtin::layout::{CenteredMain, Grid, MainAndStack},
    extensions::layout::{Fibonacci, Tatami},
    pure::geometry::Rect,
    stack,
};
use penrose_ui::layout_viewer::LayoutViewer;

const BLACK: u32 = 0x252535ff; // #252535
const WHITE: u32 = 0xdcd7baff; // #dcd7ba
const BLUE: u32 = 0x658594ff; //  #658594
pub const RED: u32 = 0xc34043ff; //   #C34043
const R: Rect = Rect::new(0, 0, 640, 480);
const FRAME_MS: u64 = 200;
const GPX: u32 = 5;

fn main() -> anyhow::Result<()> {
    let layouts = vec![
        MainAndStack::boxed_default(),
        MainAndStack::boxed_default_rotated(),
        CenteredMain::boxed_default(),
        CenteredMain::boxed_default_rotated(),
        Fibonacci::boxed_default(),
        Tatami::boxed_default(),
        Grid::boxed(),
    ];

    let mut v = LayoutViewer::new(R, BLACK, BLUE, WHITE, RED)?;
    let s = stack!(1, 2, 3, 4, 5, 6).map(Into::into);

    loop {
        v.showcase_layouts(s.clone(), &layouts, GPX, FRAME_MS)?;
    }
}
