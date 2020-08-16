use std::{thread, time};

use anyhow;

use penrose::draw::*;

const HEIGHT: usize = 20;

const PROFONT: &'static str = "ProFont For Powerline";
const SERIF: &'static str = "Serif";
const FIRA: &'static str = "Fira Code";

const BLACK: u32 = 0x282828;
const WHITE: u32 = 0xebdbb2;
const RED: u32 = 0xcc241d;
const PURPLE: u32 = 0xb16286;

fn main() -> anyhow::Result<()> {
    let mut drw = XCBDraw::new()?;
    let (w, _) = drw.screen_size(0)?;
    let id = drw.new_window(&WindowType::Dock, w, HEIGHT)?;
    drw.register_font(PROFONT);
    drw.register_font(SERIF);
    drw.register_font(FIRA);

    let mut ctx = drw.context_for(id)?;

    ctx.color(WHITE);
    ctx.rectangle(0.0, 0.0, w as f64, HEIGHT as f64);
    ctx.translate(1.0, 1.0);

    ctx.color(BLACK);
    ctx.font(PROFONT, 12)?;
    let (offset, _) = ctx.text("this is a simple test", (0.0, 8.0, 0.0, 0.0))?;

    ctx.color(RED);
    ctx.font(SERIF, 10)?;
    ctx.translate((offset + 5) as f64, 0.0);
    let (offset, _) = ctx.text("BORK BORK!", (0.0, 0.0, 0.0, 0.0))?;

    ctx.color(PURPLE);
    ctx.font(FIRA, 10)?;
    ctx.translate((offset + 5) as f64, 0.0);
    ctx.text("Look at all the colors!", (0.0, 0.0, 0.0, 0.0))?;

    drw.flush();

    thread::sleep(time::Duration::from_millis(5000));
    Ok(())
}
