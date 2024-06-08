use penrose::{
    pure::geometry::{Point, Rect},
    x::{Atom, WinType, XConn},
    x11rb::RustConn,
};
use penrose_ui::Draw;

pub const BLACK: u32 = 0x252535ff; // #252535
pub const WHITE: u32 = 0xdcd7baff; // #dcd7ba
pub const GREY: u32 = 0x363646ff; //  #363646
pub const BLUE: u32 = 0x658594ff; //  #658594

const DX: u32 = 100;
const DY: u32 = 100;
const W: u32 = 600;
const H: u32 = 60;
const FONT: &str = "mono";

fn main() -> anyhow::Result<()> {
    let conn = RustConn::new()?;
    let screen_rects = conn.screen_details()?;
    let Rect { x, y, .. } = screen_rects.last().unwrap();

    let mut drw = Draw::new(FONT, 14, BLACK)?;
    let w = drw.new_window(
        WinType::InputOutput(Atom::NetWindowTypeDock),
        Rect::new(x + DX, y + DY, W, H),
        false,
    )?;

    let r = Rect {
        x: 0,
        y: 0,
        w: W,
        h: H,
    };

    let mut ctx = drw.context_for(w)?;

    for n in 0..4 {
        let color = if n % 2 == 0 { BLUE } else { WHITE };

        ctx.set_x_offset(0);
        ctx.fill_rect(r, color.into())?;
        ctx.fill_polygon(
            &[Point::new(0, 0), Point::new(H, 0), Point::new(0, H)],
            GREY.into(),
        )?;
        ctx.set_x_offset((W - H) as i32);
        ctx.fill_polygon(
            &[Point::new(0, H), Point::new(H, 0), Point::new(H, H)],
            GREY.into(),
        )?;
        ctx.flush();
        conn.map(w)?;

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}
