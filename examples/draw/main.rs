use std::{thread, time};

use penrose::{core::hooks::Hook, draw::*, Config, Result, WindowManager, XcbConnection};

const HEIGHT: usize = 18;

const PROFONT: &str = "ProFont For Powerline";
const FIRA: &str = "Fira Code";
const SERIF: &str = "Serif";

const BLACK: Color = Color::new_from_hex(0x2828_28FF);
const GREY: Color = Color::new_from_hex(0x3C38_36FF);
const WHITE: Color = Color::new_from_hex(0xEBDB_B2FF);
const PURPLE: Color = Color::new_from_hex(0xB162_86FF);
const BLUE: Color = Color::new_from_hex(0x4585_88FF);
const RED: Color = Color::new_from_hex(0xCC24_1DFF);

const STYLE: TextStyle = TextStyle {
    font: PROFONT,
    point_size: 11,
    fg: WHITE,
    bg: Some(BLACK),
    padding: (2.0, 2.0),
};

fn main() -> Result<()> {
    bar_draw()?;
    simple_draw()?;
    Ok(())
}

fn bar_draw() -> Result<()> {
    let workspaces = &["1", "2", "3", "4", "5", "6"];
    let highlight = BLUE;
    let empty_ws = GREY;
    let mut bar = dwm_bar(
        Box::new(XCBDraw::new()?),
        HEIGHT,
        &STYLE,
        highlight,
        empty_ws,
        workspaces,
    )?;

    let config = Config::default();
    let conn = XcbConnection::new()?;
    let mut wm = WindowManager::init(config, &conn);
    bar.startup(&mut wm); // ensure widgets are initialised correctly

    thread::sleep(time::Duration::from_millis(1000));
    for focused in 1..6 {
        bar.workspace_change(&mut wm, focused - 1, focused);
        bar.event_handled(&mut wm);
        thread::sleep(time::Duration::from_millis(1000));
    }

    thread::sleep(time::Duration::from_millis(10000));
    Ok(())
}

fn simple_draw() -> Result<()> {
    let mut drw = XCBDraw::new()?;
    let (_, _, w, _) = drw.screen_sizes()?[0].values();
    let id = drw.new_window(&WindowType::Dock, 0, 0, w as usize, HEIGHT)?;
    drw.register_font(PROFONT);
    drw.register_font(SERIF);
    drw.register_font(FIRA);

    let mut ctx = drw.context_for(id)?;

    ctx.color(&WHITE);
    ctx.rectangle(0.0, 0.0, w as f64, HEIGHT as f64);
    ctx.translate(1.0, 1.0);

    ctx.color(&BLACK);
    ctx.font(PROFONT, 12)?;
    let (offset, _) = ctx.text("this is a simple test", 0.0, (0.0, 8.0))?;

    ctx.color(&RED);
    ctx.font(SERIF, 10)?;
    ctx.translate((offset + 5.0) as f64, 0.0);
    let (offset, _) = ctx.text("BORK BORK!", 0.0, (0.0, 0.0))?;

    ctx.color(&PURPLE);
    ctx.font(FIRA, 10)?;
    ctx.translate((offset + 5.0) as f64, 0.0);
    ctx.text("Look at all the colors!", 0.0, (0.0, 0.0))?;

    drw.flush(id);
    thread::sleep(time::Duration::from_millis(5000));
    Ok(())
}
