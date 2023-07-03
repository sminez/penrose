//! Utility functions for use in other parts of penrose
use crate::{
    core::layout::Layout,
    pure::{geometry::Rect, Stack},
    Result, Xid,
};
use std::{
    io::Read,
    process::{Command, Stdio},
};
use tracing::debug;

/// Run an external command
///
/// This redirects the process stdout and stderr to /dev/null.
pub fn spawn<S: Into<String>>(cmd: S) -> Result<()> {
    let s = cmd.into();
    let parts: Vec<&str> = s.split_whitespace().collect();
    let result = if parts.len() > 1 {
        Command::new(parts[0])
            .args(&parts[1..])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    } else {
        Command::new(parts[0])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Run an external command with the specified command line arguments
///
/// This redirects the process stdout and stderr to /dev/null.
pub fn spawn_with_args<S: Into<String>>(cmd: S, args: &[&str]) -> Result<()> {
    let result = Command::new(cmd.into())
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// Run an external command and return its output.
///
/// > [`std::process::Command::output`] will not work within penrose due to the
/// > way that signal handling is set up. Use this function if you need to access the
/// > output of a process that you spawn.
pub fn spawn_for_output<S: Into<String>>(cmd: S) -> std::io::Result<String> {
    let cmd = cmd.into();
    debug!(?cmd, "spawning subprocess for output");
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let result = if parts.len() > 1 {
        Command::new(parts[0])
            .stdout(Stdio::piped())
            .args(&parts[1..])
            .spawn()
    } else {
        Command::new(parts[0]).stdout(Stdio::piped()).spawn()
    };

    debug!(?cmd, "reading output");
    let mut child = result?;
    let mut buff = String::new();
    child
        .stdout
        .take()
        .expect("to have output")
        .read_to_string(&mut buff)
        .map(|_| buff)
}

/// Run an external command with arguments and return its output.
///
/// > [`std::process::Command::output`] will not work within penrose due to the
/// > way that signal handling is set up. Use this function if you need to access the
/// > output of a process that you spawn.
pub fn spawn_for_output_with_args<S: Into<String>>(
    cmd: S,
    args: &[&str],
) -> std::io::Result<String> {
    let cmd = cmd.into();

    debug!(?cmd, ?args, "spawning subprocess for output");
    let mut child = Command::new(&cmd)
        .stdout(Stdio::piped())
        .args(args)
        .spawn()?;

    debug!(?cmd, ?args, "reading output");
    let mut buff = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut buff)
        .map(|_| buff)
}

/// Use `notify-send` to display a message to the user
pub fn notify(msg: &str) -> std::io::Result<()> {
    Command::new("notify-send").arg(msg).output().map(|_| ())
}

/// Run a given [`Layout`] for a stack of n clients and print a simple ASCII rendering
/// of the resulting client positions.
///
/// This is provided as a helper function to make it easier to develop your own layout
/// functions and see the results without needing to make use of them directly in your
/// window manager. The output shown below is obtained by running the [MainAndStack][0]
/// layout with a screen size of (40, 15) and 4 clients.
///
/// ### A note on screen dimensions
/// In order to print out the ASCII representation of your layout, the "screen" dimensions
/// being used will be significantly smaller than a typical real-life monitor. You may
/// observe some unexpected behaviour from your layout when running this function if you
/// have any hard coded values relating to screen size or individual client sizes.
///
/// ```text
/// .........................................
/// .                       .               .
/// .                       .               .
/// .                       .               .
/// .                       .               .
/// .                       .................
/// .                       .               .
/// .                       .               .
/// .                       .               .
/// .                       .               .
/// .                       .................
/// .                       .               .
/// .                       .               .
/// .                       .               .
/// .                       .               .
/// .........................................
/// ```
///
/// [0]: crate::builtin::layout::MainAndStack
pub fn print_layout_result<L: Layout>(
    l: &mut L,
    n_clients: u32,
    screen_width: u32,
    screen_height: u32,
) {
    let s: Stack<Xid> = Stack::try_from_iter((0..n_clients).map(Into::into)).expect("non-empty");
    let (_, positions) = l.layout(&s, Rect::new(0, 0, screen_width, screen_height));

    let mut screen = vec![vec![' '; (screen_width + 1) as usize]; (screen_height + 1) as usize];
    for (_, Rect { x, y, w, h }) in positions.into_iter() {
        for i in 0..=w {
            screen[y as usize][(x + i) as usize] = '.';
            screen[(y + h) as usize][(x + i) as usize] = '.';
        }
        for i in 0..=h {
            screen[(y + i) as usize][x as usize] = '.';
            screen[(y + i) as usize][(x + w) as usize] = '.';
        }
    }

    for row in screen.into_iter() {
        let chars: String = row.into_iter().collect();
        println!("{chars}");
    }
}
