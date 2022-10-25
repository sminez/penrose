//! Helpers and pre-defined actions for use in user defined key bindings
use crate::{
    core::{
        actions::{key_handler, modify_with},
        bindings::KeyEventHandler,
        layout::LayoutStack,
        State,
    },
    pure::RelativePosition,
    util::{spawn, spawn_for_output, spawn_with_args},
    x::{atom::Atom, property::Prop, XConn, XConnExt},
    Error, Result,
};
use tracing::{error, info};

/// Exit penrose
///
/// Immediately exit the window manager with exit code 0.
pub fn exit<X>() -> Box<dyn KeyEventHandler<X>>
where
    X: XConn,
{
    key_handler(|_, _| std::process::exit(0))
}

/// Info log the current window manager [State].
pub fn log_current_state<X>() -> Box<dyn KeyEventHandler<X>>
where
    X: XConn + std::fmt::Debug,
{
    key_handler(|s: &mut State<X>, _| {
        info!("Current Window Manager State: {s:#?}");
        Ok(())
    })
}

/// Jump to, or create, a [Workspace]
///
/// Call 'get_name' to obtain a Workspace name and check to see if there is currently a Workspace
/// with that name being managed by the WindowManager. If there is no existing workspace with the
/// given name, create it with the supplied available layouts. If a matching Workspace _does_
/// already exist then simply switch focus to it. This action is most useful when combined with the
/// DefaultWorkspace hook that allows for auto populating named Workspaces when first focusing them.
pub fn create_or_switch_to_workspace<X>(
    get_name: fn() -> Option<String>,
    layouts: LayoutStack,
) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn + std::fmt::Debug,
{
    modify_with(move |cs| {
        if let Some(name) = get_name() {
            if !cs.contains_tag(&name) {
                cs.add_workspace(&name, layouts.clone());
            }

            cs.focus_tag(&name);
        }
    })
}

/// Focus a client with the given class as `WM_CLASS` or spawn the program with the given command
/// if no such client exists.
///
/// This is useful for key bindings that are based on the program you want to work with rather than
/// having to remember where things are running.
pub fn focus_or_spawn<X>(class: &'static str, command: &'static str) -> Box<dyn KeyEventHandler<X>>
where
    X: XConn + std::fmt::Debug,
{
    key_handler(move |s: &mut State<X>, x: &X| {
        let mut client = None;

        for &id in s.client_set.iter_clients() {
            if let Some(Prop::UTF8String(classes)) = x.get_prop(id, Atom::WmClass.as_ref())? {
                if classes.iter().any(|s| s == class) {
                    client = Some(id);
                    break;
                }
            }
        }

        x.modify_and_refresh(s, |cs| {
            if let Some(id) = client {
                cs.focus_client(&id)
            } else if let Err(e) = spawn(command) {
                error!(%e, %command, "unable to spawn program")
            }
        })
    })
}

/// Detect the current monitor set up and arrange the monitors if needed using [xrandr][1].
///
/// NOTE
/// - Primary monitor will be set to `primary`
/// - Monitor resolution is set using the --auto flag in xrandr
/// - Only supports one and two monitor setups.
///
/// [1]: https://wiki.archlinux.org/index.php/Xrandr
pub fn update_monitors_via_xrandr(
    primary: &str,
    secondary: &str,
    position: RelativePosition,
) -> Result<()> {
    let raw = spawn_for_output("xrandr")?;
    let status = raw
        .lines()
        .find(|line| line.starts_with(secondary))
        .ok_or_else(|| {
            Error::Custom("unable to find secondary monitor in xrandr output".to_owned())
        })?
        .split(' ')
        .nth(1)
        .ok_or_else(|| Error::Custom("unexpected xrandr output".to_owned()))?;

    let pos = match position {
        RelativePosition::Left => "--left-of",
        RelativePosition::Right => "--right-of",
        RelativePosition::Above => "--above",
        RelativePosition::Below => "--below",
    };

    // force the primary monitor
    spawn(format!("xrandr --output {} --primary --auto", primary))?;

    // Updated the secondary monitor
    match status {
        "disconnected" => spawn(format!("xrandr --output {secondary} --off")),

        "connected" => spawn(format!(
            "xrandr --output {secondary} --auto {pos} {primary}",
        )),

        _ => Ok(()),
    }
}

/// A notification level when calling notify-send
///
/// The effect of this on the generated notification will depend on the notification
/// daemon that you are using.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NotifyLevel {
    Low,
    Normal,
    Critical,
}

/// Send a notification using the `notify-send` command line program
pub fn notify_send(title: impl AsRef<str>, body: impl AsRef<str>) -> Result<()> {
    notify_send_custom(title, body, NotifyLevel::Normal, 5000)
}

/// Send a notification using the `notify-send` command line program
///
/// Duration is in ms.
pub fn notify_send_custom(
    title: impl AsRef<str>,
    body: impl AsRef<str>,
    level: NotifyLevel,
    duration: usize,
) -> Result<()> {
    let level = match level {
        NotifyLevel::Low => "low",
        NotifyLevel::Normal => "normal",
        NotifyLevel::Critical => "critical",
    };

    spawn_with_args(
        "notify-send",
        &[
            "-u",
            level,
            "-t",
            &duration.to_string(),
            title.as_ref(),
            body.as_ref(),
        ],
    )
}
