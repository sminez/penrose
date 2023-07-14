//! Demo of the text rendering API
use penrose::{
    pure::geometry::Rect,
    x::{Atom, WinType},
    Color,
};
use penrose_ui::Draw;
use std::{thread::sleep, time::Duration};

const DX: u32 = 100;
const DY: u32 = 100;
const W: u32 = 500;
const H: u32 = 60;
const FONT: &str = "ProFont For Powerline";
const TXT: &str = "    text is great! ◈ ζ ᛄ ℚ";

fn main() -> anyhow::Result<()> {
    let fg1 = Color::try_from("#fad07b")?;
    let fg2 = Color::try_from("#458588")?;
    let fg3 = Color::try_from("#a6cc70")?;
    let fg4 = Color::try_from("#b16286")?;
    let bg = Color::try_from("#282828")?;

    let mut drw = Draw::new(FONT, 12, bg)?;
    let w = drw.new_window(
        WinType::InputOutput(Atom::NetWindowTypeDock),
        Rect::new(DX, DY, W, H),
        false,
    )?;

    let mut ctx = drw.context_for(w)?;
    ctx.clear()?;

    let (dx, dy) = ctx.draw_text(TXT, 0, (10, 0), fg1)?;

    ctx.set_x_offset(dx as i32 + 10);
    ctx.draw_text(TXT, 0, (5, 0), fg2)?;

    ctx.translate(0, dy as i32 + 10);
    ctx.draw_text(TXT, 0, (5, 0), fg3)?;

    ctx.translate(-(dx as i32), 0);
    ctx.draw_text(TXT, 0, (0, 0), fg4)?;

    ctx.flush();
    drw.flush(w)?;

    sleep(Duration::from_secs(2));

    Ok(())
}
