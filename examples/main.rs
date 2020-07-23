/**
 * penrose :: example configuration
 *
 * penrose does not have a traditional configuration file and is not typically set up by patching
 * the source code: it is more like Xmonad or Qtile in the sense that it is really a library for
 * writing your own window manager. Below is an example main.rs that can serve as a template should
 * you decide to write your own WM using penrose.
 */
#[macro_use]
extern crate penrose;

use penrose::layout::{bottom_stack, paper, side_stack};
use penrose::{
    Backward, ColorScheme, Config, Forward, Layout, LayoutConf, Less, More, WindowManager,
    XcbConnection,
};

fn main() {
    // penrose will log useful information about the current state of the WindowManager during
    // normal operation that can be used to drive scripts and related programs. Additional debug
    // output can be helpful if you are hitting issues.
    // NOTE: you can include a logging handler such as simplelog shown below to see the logging output
    // simplelog::SimpleLogger::init(simplelog::LevelFilter::Info, simplelog::Config::default()).unwrap();

    // Created at startup. See keybindings below for how to access them
    let workspaces = &["1", "2", "3", "4", "5", "6", "7", "8", "9"];

    // Windows with a matching WM_CLASS will always float
    let floating_classes = &["rofi", "dmenu", "dunst", "polybar", "pinentry-gtk-2"];

    // Only the highlight color is currently used (window borders). Work is planned for an embedded
    // task bar and systray which will make use of the other colors
    let color_scheme = ColorScheme {
        bg: 0x282828,        // #282828
        fg_1: 0x3c3836,      // #3c3836
        fg_2: 0xa89984,      // #a89984
        fg_3: 0xf2e5bc,      // #f2e5bc
        highlight: 0xcc241d, // #cc241d
        urgent: 0x458588,    // #458588
    };

    // When specifying a layout, most of the time you will want LayoutConf::default() as shown
    // below, which will honour gap settings and will not be run on focus changes (only when
    // clients are added/removed). To customise when/how each layout is applied you can create a
    // LayoutConf instance with your desired properties enabled.
    let follow_focus_conf = LayoutConf {
        floating: false,
        gapless: true,
        follow_focus: true,
    };

    // Defauly number of clients in the main layout area
    let n_main = 1;

    // Default percentage of the screen to fill with the main area of the layout
    let ratio = 0.6;

    // Layouts to be used on each workspace. Currently all workspaces have the same set of Layouts
    // available to them, though they track modifications to n_main and ratio independently.
    let layouts = vec![
        Layout::new("[side]", LayoutConf::default(), side_stack, n_main, ratio),
        Layout::new("[botm]", LayoutConf::default(), bottom_stack, n_main, ratio),
        Layout::new("[papr]", follow_focus_conf, paper, n_main, ratio),
        Layout::floating("[----]"),
    ];

    // Set the root X window name to be the active layout symbol so it can be picked up by external
    // programs such as a status bar or script.
    let active_layout_as_root_name = |wm: &mut WindowManager| {
        wm.set_root_window_name(wm.current_layout_symbol());
    };

    // The gen_keybindings macro parses user friendly key binding definitions into X keycodes and
    // modifier masks. It uses the 'xmodmap' program to determine your current keymap and create
    // the bindings dynamically on startup. If this feels a little too magical then you can
    // alternatively construct a  HashMap<KeyCode, FireAndForget> manually with your chosen
    // keybindings (see helpers.rs and data_types.rs for details).
    // FireAndForget functions do not need to make use of the mutable WindowManager reference they
    // are passed if it is not required: the run_external macro ignores the WindowManager itself
    // and instead spawns a new child process, returning an Option<Child> so that penrose can clean
    // up the process when it exits. If no child processes are spawned (in the case of running
    // methods on the WindowManager for example) then None must be returned instead.
    let key_bindings = gen_keybindings! {
        // Program launch
        "M-semicolon" => run_external!("dmenu"),
        "M-Return" => run_external!("xterm"),

        // actions
        "M-A-s" => run_external!("screenshot"),
        "M-A-k" => run_external!("toggle-kb-for-tada"),
        "M-A-l" => run_external!("lock-screen"),
        "M-A-m" => run_external!("xrandr --output HDMI-1 --auto --right-of eDP-1 "),

        // client management
        "M-j" => run_internal!(cycle_client, Forward),
        "M-k" => run_internal!(cycle_client, Backward),
        "M-S-j" => run_internal!(drag_client, Forward),
        "M-S-k" => run_internal!(drag_client, Backward),
        "M-S-q" => run_internal!(kill_client),

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace),
        "M-bracketright" => run_internal!(cycle_screen, Forward),
        "M-bracketleft" => run_internal!(cycle_screen, Backward),
        "M-S-bracketright" => run_internal!(drag_workspace, Forward),
        "M-S-bracketleft" => run_internal!(drag_workspace, Backward),

        // Layout & window management
        "M-grave" => Box::new(move |wm: &mut WindowManager| {
            wm.cycle_layout(Forward);
            active_layout_as_root_name(wm);
        }),
        "M-S-grave" => Box::new(move |wm: &mut WindowManager| {
            wm.cycle_layout(Backward);
            active_layout_as_root_name(wm);
        }),
        "M-A-Up" => run_internal!(update_max_main, More),
        "M-A-Down" => run_internal!(update_max_main, Less),
        "M-A-Right" => run_internal!(update_main_ratio, More),
        "M-A-Left" => run_internal!(update_main_ratio, Less),
        "M-A-Escape" => run_internal!(exit);

        // Each keybinding here will be templated in with the workspace index of each workspace,
        // allowing for common workspace actions to be bound at once.
        forall_workspaces: workspaces => {
            "M-{}" => focus_workspace,
            "M-S-{}" => client_to_workspace,
        }
    };

    // The underlying connection to the X server is handled as a trait: XConn. XcbConnection is the
    // reference implementation of this trait that uses the XCB library to communicate with the X
    // server. You are free to provide your own implementation if you wish, see xconnection.rs for
    // details of the required methods and expected behaviour.
    let conn = XcbConnection::new();

    let mut wm = WindowManager::init(
        Config {
            workspaces: workspaces,
            fonts: &[],
            floating_classes: floating_classes,
            layouts: layouts,
            color_scheme: color_scheme,
            border_px: 2,
            gap_px: 5,
            main_ratio_step: 0.05,
            systray_spacing_px: 2,
            show_systray: true,
            show_bar: true,
            top_bar: true,
            bar_height: 18,
            respect_resize_hints: true,
        },
        &conn,
    );

    // A startup script can be run as follows
    // spawn(format!(
    //     "{}/bin/scripts/penrose-startup.sh",
    //     env::var("HOME").unwrap()
    // ));

    // Call out custom hook with the new WindowManager instance to set the X root window name.
    active_layout_as_root_name(&mut wm);

    // grab_keys_and_run will start listening to events from the X server and drop into the main
    // event loop. From this point on, program control passes to the WindowManager so make sure
    // that any logic you wish to run is done before here!
    wm.grab_keys_and_run(key_bindings);
}
