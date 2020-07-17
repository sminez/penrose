<image width="100px" src="icon.svg" align="left"></image>
penrose - a tiling window manager library
=========================================

[![Build](https://github.com/sminez/penrose/workflows/Build/badge.svg?branch=master)](https://github.com/sminez/penrose/actions?query=workflow%3ABuild) [![crates.io version](https://img.shields.io/crates/v/penrose)](https://crates.io/crates/penrose)

`penrose` is a library for configuring your own X11 window
manager in Rust. It uses a workspace model (as opposed to tags) and is built on
top of the XCB API.  The project is very much a work in progress as I try to set
up my ideal, minimal window manager for daily use. `penrose` is now feature
complete enough and stable enough to use as your primary Window Manager, so long
as you don't mind a few rough edges! I am aiming for the code to be well
documented and easy to extend. For now, this is my primary hobby project so
updates are frequent: I try to keep crates.io up to date but please check the
git repo for latest changes.

![screenshot](screenshot.png)

### Current project status
If you don't mind a bare bones (_really_ bare bones) window manager then you can
take penrose for a spin using the config in the `example` directory. You will
need to update the keybindings to launch your preferred terminal emulator and
program launcher and may want to adjust the floating window classes to handle
some additional programs.

#### Current functionality
- partial EWMH support (active window/desktop, number of desktops, desktop
  names, window manager name, desktop for client)
- user defined layout functions (`side_stack` implemented)
- layout resizing and modification
- configurable window borders for focused / unfocused
- configurable gaps
- configurable keybindings (internal methods and external programs)
- workspaces
- moving clients between workspaces
- kill focused client
- cycle focus
- drag focused client through stack
- floating windows
- ...


### Project Non-goals
- A config file
  - Parsing a config file and dynamically switching behaviour on the contents
  adds a huge amount of complexity to the code. I'd much rather keep the code
  simple and well documented so that modifying it is easy and then just
  recompile.

- IPC / relying on external programs
  - I love acme from plan9 and how easy it is to drive it's state from external
  programs (check out my [acme-corp](https://github.com/sminez/acme-corp) tools
  to see what I mean) but that comes at the expense of the internal logic
  becoming _massively_ more complicated. As few moving parts as possible is
  ideal. So, things that are easy to acomplish using the XCB api (key bindings,
  simple rendering of a bar etc) are in, full on IPC via an exposed API is out.


### Current Work
See the [TODO](TODO) file in the root of the repo for next steps and ongoing work. I'm
trying to keep it mostly up to date but the docs/TODOs and the codebase may
diverge at points when I forget to update things. If in doubt, read the source.
