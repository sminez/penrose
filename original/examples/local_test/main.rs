// written for penrose v0.2.0
use tracing::Level;
use tracing_subscriber::prelude::*;

use penrose::{
    common::{
        bindings::MouseEvent,
        helpers::{index_selectors, logging_error_handler},
    },
    contrib::{
        extensions::Scratchpad,
        hooks::{ClientSpawnRules, SpawnRule},
        layouts::paper,
    },
    core::{
        config::Config,
        hooks::{Hook, Hooks},
        layout::{bottom_stack, monocle, side_stack, Layout, LayoutConf},
        manager::WindowManager,
        ring::Selector,
    },
    draw::{dwm_bar, TextStyle},
    gen_keybindings, gen_mousebindings, run_external, run_internal,
    xcb::{new_xcb_backed_window_manager, Api, XcbConnection, XcbDraw},
    Backward, Forward, Less, More,
};

fn _my_layouts() -> Vec<Layout> {
    const N_MAIN: u32 = 1;
    const RATIO: f32 = 0.6;

    let mono_conf = LayoutConf {
        follow_focus: true,
        gapless: true,
        ..Default::default()
    };

    let mut paper_conf = mono_conf;
    paper_conf.allow_wrapping = false;

    vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, N_MAIN, RATIO),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, N_MAIN, RATIO),
        Layout::new("[papr]", paper_conf, paper, N_MAIN, RATIO),
        Layout::new("[mono]", mono_conf, monocle, N_MAIN, RATIO),
    ]
}

fn _run_penrose() -> penrose::Result<()> {
    const HEIGHT: usize = 18;
    const PROFONT: &str = "ProFont For Powerline";

    const BLACK: u32 = 0x282828ff;
    const GREY: u32 = 0x3c3836ff;
    const WHITE: u32 = 0xebdbb2ff;
    const BLUE: u32 = 0x458588ff;

    tracing_subscriber::fmt()
        .with_max_level(Level::TRACE)
        // .json()
        // .pretty()
        .finish()
        .init();

    let mut config_builder = Config::default().builder();
    let config = config_builder
        .floating_classes(vec!["rofi", "dmenu", "dunst", "pinentry-gtk-2"])
        .layouts(_my_layouts())
        .build()
        .expect("failed to build config");

    let sp = Scratchpad::new("st", 0.8, 0.8);

    let hooks: Hooks<_> = vec![
        sp.get_hook() as Box<dyn Hook<XcbConnection>>,
        ClientSpawnRules::new(vec![SpawnRule::ClassName("thunar", 3)]),
        Box::new(dwm_bar(
            XcbDraw::new()?,
            HEIGHT,
            &TextStyle {
                font: PROFONT.to_string(),
                point_size: 11,
                fg: WHITE.into(),
                bg: Some(BLACK.into()),
                padding: (2.0, 2.0),
            },
            BLUE,
            GREY,
            config.workspaces().clone(),
        )?),
    ];

    let key_bindings = gen_keybindings! {
        "A-C-semicolon" => run_external!("rofi-apps");
        "A-C-Return" => run_external!("st");
        "A-C-slash" => sp.toggle();

        // client management
        "A-C-j" => run_internal!(cycle_client, Forward);
        "A-C-k" => run_internal!(cycle_client, Backward);
        "A-C-S-j" => run_internal!(drag_client, Forward);
        "A-C-S-k" => run_internal!(drag_client, Backward);
        "A-C-f" => run_internal!(toggle_client_fullscreen, &Selector::Focused);
        "A-C-q" => run_internal!(kill_client);

        // workspace management
        "A-C-Tab" => run_internal!(toggle_workspace);
        "A-C-period" => run_internal!(cycle_workspace, Forward);
        "A-C-comma" => run_internal!(cycle_workspace, Backward);
        "A-C-S-bracketright" => run_internal!(drag_workspace, Forward);
        "A-C-S-bracketleft" => run_internal!(drag_workspace, Backward);

        // Layout management
        "A-C-grave" => run_internal!(cycle_layout, Forward);
        "A-C-S-grave" => run_internal!(cycle_layout, Backward);
        "A-C-Up" => run_internal!(update_max_main, More);
        "A-C-Down" => run_internal!(update_max_main, Less);
        "A-C-Right" => run_internal!(update_main_ratio, More);
        "A-C-Left" => run_internal!(update_main_ratio, Less);

        "A-C-Escape" => run_internal!(exit);

        map: { "1", "2", "3", "4", "5", "6", "7", "8", "9" } to index_selectors(9) => {
            "A-C-{}" => focus_workspace (REF);
            "A-C-S-{}" => client_to_workspace (REF);
        };
    };

    let mouse_bindings = gen_mousebindings! {
        Press Right + [Alt, Ctrl] => |wm: &mut WindowManager<_>, _: &MouseEvent| wm.cycle_workspace(Forward),
        Press Left + [Alt, Ctrl] => |wm: &mut WindowManager<_>, _: &MouseEvent| wm.cycle_workspace(Backward)
    };

    let mut wm = new_xcb_backed_window_manager(config, hooks, logging_error_handler())?;
    wm.grab_keys_and_run(key_bindings, mouse_bindings)
}

fn _check_randr_version() {
    let (a, b) = (20, 2);
    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let cookie = xcb::randr::query_version(&conn, a, b);
    let reply = cookie.get_reply().unwrap();
    let (maj, min) = (reply.major_version(), reply.minor_version());
    if (maj, min) != (a, b) {
        panic!(
            "penrose requires RandR version > 1.2: detected {}.{}",
            maj, min
        )
    }
}

fn _window_props(id: u32) -> penrose::Result<()> {
    let api = Api::new()?;
    let mut props = api.list_props(id)?;
    props.sort();
    for prop in props.iter() {
        match api.get_prop(id, prop) {
            Ok(val) => println!("{:<20} {:?}", prop, val),
            Err(e) => println!("{:<20} can't be parsed: {}", prop, e),
        }
    }
    println!("{:?}", api.get_window_attributes(id)?);

    println!();
    Ok(())
}

fn main() -> penrose::Result<()> {
    // _run_penrose()
    let api = Api::new()?;
    for id in api.current_clients()?.into_iter() {
        _window_props(id)?;
    }

    Ok(())
}
