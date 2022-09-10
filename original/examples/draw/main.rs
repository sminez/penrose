use penrose::{
    common::geometry::Region,
    core::{config::Config, hooks::Hook},
    draw::{bar::dwm_bar, Draw, DrawContext, TextStyle},
    logging_error_handler,
    xcb::{new_xcb_backed_window_manager, XcbDraw},
    xconnection::{Atom, WinType},
    Result,
};
use std::{thread, time};

const HEIGHT: usize = 18;

const PROFONT: &str = "ProFont For Powerline";
const FIRA: &str = "Fira Code";
const SERIF: &str = "Serif";

const BLACK: u32 = 0x282828ff;
const GREY: u32 = 0x3c3836ff;
const WHITE: u32 = 0xebdbb2ff;
const PURPLE: u32 = 0xb16286ff;
const BLUE: u32 = 0x458588ff;
const RED: u32 = 0xcc241dff;

fn main() -> Result<()> {
    simple_draw()?;
    bar_draw()?;
    Ok(())
}

fn bar_draw() -> Result<()> {
    let workspaces = vec!["1", "2", "3", "4", "5", "6"];
    let style = TextStyle {
        font: PROFONT.to_string(),
        point_size: 11,
        fg: WHITE.into(),
        bg: Some(BLACK.into()),
        padding: (2.0, 2.0),
    };
    let highlight = BLUE;
    let empty_ws = GREY;
    let mut bar = dwm_bar(
        XcbDraw::new()?,
        HEIGHT,
        &style,
        highlight,
        empty_ws,
        workspaces,
    )?;

    let mut wm = new_xcb_backed_window_manager(Config::default(), vec![], logging_error_handler())?;
    bar.startup(&mut wm)?; // ensure widgets are initialised correctly

    thread::sleep(time::Duration::from_millis(1000));
    for focused in 1..6 {
        bar.workspace_change(&mut wm, focused - 1, focused)?;
        bar.event_handled(&mut wm)?;
        thread::sleep(time::Duration::from_millis(1000));
    }

    thread::sleep(time::Duration::from_millis(10000));
    Ok(())
}

fn simple_draw() -> Result<()> {
    let mut drw = XcbDraw::new()?;
    let (_, _, w, _) = drw.screen_sizes()?[0].values();
    let id = drw.new_window(
        WinType::InputOutput(Atom::NetWindowTypeNormal),
        Region::new(0, 0, w, HEIGHT as u32),
        false,
    )?;
    drw.register_font(PROFONT);
    drw.register_font(SERIF);
    drw.register_font(FIRA);

    let mut ctx = drw.context_for(id)?;

    ctx.color(&WHITE.into());
    ctx.rectangle(0.0, 0.0, w as f64, HEIGHT as f64)?;
    ctx.translate(1.0, 1.0);

    ctx.color(&BLACK.into());
    ctx.font(PROFONT, 12)?;
    let (offset, _) = ctx.text("this is a simple test", 0.0, (0.0, 8.0))?;

    ctx.color(&RED.into());
    ctx.font(SERIF, 10)?;
    ctx.translate((offset + 5.0) as f64, 0.0);
    let (offset, _) = ctx.text("BORK BORK!", 0.0, (0.0, 0.0))?;

    ctx.color(&PURPLE.into());
    ctx.font(FIRA, 10)?;
    ctx.translate((offset + 5.0) as f64, 0.0);
    ctx.text("Look at all the colors!", 0.0, (0.0, 0.0))?;

    drw.flush(id)?;
    thread::sleep(time::Duration::from_millis(5000));
    Ok(())
}
