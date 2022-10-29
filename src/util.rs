//! Utility functions for use in other parts of penrose
use crate::Result;
use std::{
    io::Read,
    process::{Command, Stdio},
};
use tracing::info;

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
pub fn spawn_for_output<S: Into<String>>(cmd: S) -> std::io::Result<String> {
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
/// NOTE: std::process::Command::output will not work within penrose due to the
/// way that signal handling is set up. Use this function if you need to access the
/// output of a process that you spawn.
pub fn spawn_for_output_with_args<S: Into<String>>(
    cmd: S,
    args: &[&str],
) -> std::io::Result<String> {
    let cmd = cmd.into();

    info!(?cmd, ?args, "spawning subprocess for output");
    let mut child = Command::new(&cmd)
        .stdout(Stdio::piped())
        .args(args)
        .spawn()?;

    info!(?cmd, ?args, "reading output");
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
