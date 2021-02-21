//! Utility functions for use in other parts of penrose
use crate::{
    core::{bindings::CodeMap, ring::Selector},
    ErrorHandler, PenroseError, Result,
};

use std::{
    io::Read,
    process::{Command, Stdio},
};

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
/// NOTE: std::process::Command::output will not work within penrose due to the
/// way that signal handling is set up. Use this function if you need to access the
/// output of a process that you spawn.
pub fn spawn_for_output<S: Into<String>>(cmd: S) -> Result<String> {
    let cmd = cmd.into();
    info!(?cmd, "spawning subprocess for output");
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let result = if parts.len() > 1 {
        Command::new(parts[0])
            .stdout(Stdio::piped())
            .args(&parts[1..])
            .spawn()
    } else {
        Command::new(parts[0]).stdout(Stdio::piped()).spawn()
    };

    let child = result?;
    let mut buff = String::new();
    Ok(child
        .stdout
        .ok_or(PenroseError::SpawnProc(cmd))?
        .read_to_string(&mut buff)
        .map(|_| buff)?)
}

/// Run an external command with arguments and return its output.
///
/// NOTE: std::process::Command::output will not work within penrose due to the
/// way that signal handling is set up. Use this function if you need to access the
/// output of a process that you spawn.
pub fn spawn_for_output_with_args<S: Into<String>>(cmd: S, args: &[&str]) -> Result<String> {
    let cmd = cmd.into();

    info!(?cmd, ?args, "spawning subprocess for output");
    let child = Command::new(&cmd)
        .stdout(Stdio::piped())
        .args(args)
        .spawn()?;

    info!(?cmd, ?args, "reading output");
    let mut buff = String::new();
    Ok(child
        .stdout
        .ok_or(PenroseError::SpawnProc(cmd))?
        .read_to_string(&mut buff)
        .map(|_| buff)?)
}

/// Run the xmodmap command to dump the system keymap table.
///
/// This is done in a form that we can load in and convert back to key
/// codes. This lets the user define key bindings in the way that they
/// would expect while also ensuring that it is east to debug any odd
/// issues with bindings by referring the user to the xmodmap output.
///
/// # Panics
/// This function will panic if it is unable to fetch keycodes using the xmodmap
/// binary on your system or if the output of `xmodmap -pke` is not valid
pub fn keycodes_from_xmodmap() -> CodeMap {
    match Command::new("xmodmap").arg("-pke").output() {
        Err(e) => panic!("unable to fetch keycodes via xmodmap: {}", e),
        Ok(o) => match String::from_utf8(o.stdout) {
            Err(e) => panic!("invalid utf8 from xmodmap: {}", e),
            Ok(s) => s
                .lines()
                .flat_map(|l| {
                    let mut words = l.split_whitespace(); // keycode <code> = <names ...>
                    let key_code: u8 = match words.nth(1) {
                        Some(word) => match word.parse() {
                            Ok(val) => val,
                            Err(e) => panic!("{}", e),
                        },
                        None => panic!("unexpected output format from xmodmap -pke"),
                    };
                    words.skip(1).map(move |name| (name.into(), key_code))
                })
                .collect::<CodeMap>(),
        },
    }
}

/// Create a Vec of index selectors for the given input slice
pub fn index_selectors<'a, T>(len: usize) -> Vec<Selector<'a, T>> {
    (0..len).map(Selector::Index).collect()
}

/// A simple error handler that just logs the error to the penrose log stream
pub fn logging_error_handler() -> ErrorHandler {
    Box::new(|e: PenroseError| error!("{}", e))
}
