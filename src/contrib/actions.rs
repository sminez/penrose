//! Additional helper functions and actions for use with penrose.
use crate::{
    core::{
        bindings::KeyEventHandler,
        client::Client,
        data_types::RelativePosition,
        helpers::{spawn, spawn_for_output},
        layout::Layout,
        manager::WindowManager,
        ring::Selector,
        workspace::Workspace,
        xconnection::XConn,
    },
    Result,
};

/**
 * Jump to, or create, a [Workspace]
 *
 * Call 'get_name' to obtain a Workspace name and check to see if there is currently a Workspace
 * with that name being managed by the WindowManager. If there is no existing workspace with the
 * given name, create it with the supplied available layouts. If a matching Workspace _does_
 * already exist then simply switch focus to it. This action is most useful when combined with the
 * DefaultWorkspace hook that allows for auto populating named Workspaces when first focusing them.
 */
pub fn create_or_switch_to_workspace<X: XConn>(
    get_name: fn() -> Option<String>,
    layouts: Vec<Layout>,
) -> KeyEventHandler<X> {
    Box::new(move |wm: &mut WindowManager<X>| {
        if let Some(s) = get_name() {
            let name = &s;
            let cond = |ws: &Workspace| ws.name() == name;
            let sel = Selector::Condition(&cond);
            if wm.workspace(&sel).is_none() {
                wm.push_workspace(Workspace::new(name, layouts.clone()))?;
            }
            wm.focus_workspace(&sel)
        } else {
            Ok(())
        }
    })
}

/**
 * Focus a [Client] with the given class as `WM_CLASS` or spawn the program with the given command
 * if no such Client exists.
 *
 * This is useful for key bindings that are based on the program you want to work with rather than
 * having to remember where things are running.
 */
pub fn focus_or_spawn<X: XConn>(
    class: impl Into<String>,
    command: impl Into<String>,
) -> KeyEventHandler<X> {
    let (class, command) = (class.into(), command.into());

    Box::new(move |wm: &mut WindowManager<X>| {
        let cond = |c: &Client| c.class() == class;
        if let Some(client) = wm.client(&Selector::Condition(&cond)) {
            let workspace = client.workspace();
            wm.focus_workspace(&Selector::Index(workspace))
        } else {
            spawn(&command)
        }
    })
}

/**
 * Detect the current monitor set up and arrange the monitors if needed using [xrandr][1].
 *
 * NOTE
 * - Primary monitor will be set to `primary`
 * - Monitor resolution is set using the --auto flag in xrandr
 * - Only supports one and two monitor setups.
 *
 * [1]: https://wiki.archlinux.org/index.php/Xrandr
 */
pub fn update_monitors_via_xrandr(
    primary: &str,
    secondary: &str,
    position: RelativePosition,
) -> Result<()> {
    let raw = spawn_for_output("xrandr")?;
    let secondary_line = raw
        .lines()
        .find(|line| line.starts_with(secondary))
        .ok_or_else(|| perror!("unable to find secondary monitor in xrandr output"))?;
    let status = secondary_line
        .split(' ')
        .nth(1)
        .ok_or_else(|| perror!("unexpected xrandr output"))?;

    let position_flag = match position {
        RelativePosition::Left => "--left-of",
        RelativePosition::Right => "--right-of",
        RelativePosition::Above => "--above",
        RelativePosition::Below => "--below",
    };

    // force the primary monitor
    spawn(format!("xrandr --output {} --primary --auto", primary))?;

    match status {
        "disconnected" => spawn(format!("xrandr --output {} --off", secondary)),
        "connected" => spawn(format!(
            "xrandr --output {} --auto {} {}",
            secondary, position_flag, primary
        )),
        _ => Ok(()),
    }
}
