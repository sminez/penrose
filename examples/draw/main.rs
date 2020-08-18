use std::{thread, time};

use penrose::{draw::*, Result};

const HEIGHT: usize = 20;

const PROFONT: &'static str = "ProFont For Powerline";
const SERIF: &'static str = "Serif";
const FIRA: &'static str = "Fira Code";

const BLACK: u32 = 0x282828;
const GREY: u32 = 0x3c3836;
const WHITE: u32 = 0xebdbb2;
const PURPLE: u32 = 0xb16286;
const BLUE: u32 = 0x458588;
const RED: u32 = 0xcc241d;

fn main() -> Result<()> {
    bar_draw()?;
    simple_draw()?;
    Ok(())
}

fn bar_draw() -> Result<()> {
    let workspaces = &["1", "2", "3", "4", "5", "6"];

    let drw = XCBDraw::new()?;
    let mut bar = StatusBar::try_new(Box::new(drw), 5.0, true, 0, HEIGHT, BLACK)?;
    bar.register_fonts(&[PROFONT, SERIF, FIRA]);
    bar.add_widget(Box::new(StaticText::new(
        "penrose",
        PROFONT,
        12,
        PURPLE,
        None,
        (0.0, 0.0, 0.0, 0.0),
        false,
    )));
    bar.add_widget(Box::new(StaticText::new(
        "test",
        PROFONT,
        10,
        RED,
        None,
        (0.0, 0.0, 0.0, 0.0),
        false,
    )));
    bar.add_widget(Box::new(WorkspaceWidget::new(
        workspaces, PROFONT, 10, 0, WHITE, GREY, BLUE, BLACK,
    )));

    bar.redraw()?;
    thread::sleep(time::Duration::from_millis(5000));
    Ok(())
}

fn simple_draw() -> Result<()> {
    let mut drw = XCBDraw::new()?;
    let (w, _) = drw.screen_size(0)?;
    let id = drw.new_window(&WindowType::Dock, 0, 0, w, HEIGHT)?;
    drw.register_font(PROFONT);
    drw.register_font(SERIF);
    drw.register_font(FIRA);

    let mut ctx = drw.context_for(id)?;

    ctx.color(&WHITE.into());
    ctx.rectangle(0.0, 0.0, w as f64, HEIGHT as f64);
    ctx.translate(1.0, 1.0);

    ctx.color(&BLACK.into());
    ctx.font(PROFONT, 12)?;
    let (offset, _) = ctx.text("this is a simple test", (0.0, 8.0, 0.0, 0.0))?;

    ctx.color(&RED.into());
    ctx.font(SERIF, 10)?;
    ctx.translate((offset + 5.0) as f64, 0.0);
    let (offset, _) = ctx.text("BORK BORK!", (0.0, 0.0, 0.0, 0.0))?;

    ctx.color(&PURPLE.into());
    ctx.font(FIRA, 10)?;
    ctx.translate((offset + 5.0) as f64, 0.0);
    ctx.text("Look at all the colors!", (0.0, 0.0, 0.0, 0.0))?;

    drw.flush();
    thread::sleep(time::Duration::from_millis(5000));
    Ok(())
}
