Penrose - a tiling window manager in the style of dwm / xmonad
==============================================================
[![Build](https://github.com/sminez/penrose/workflows/Build/badge.svg?branch=master)](https://github.com/sminez/penrose/actions?query=workflow%3ABuild) [![crates.io version](https://img.shields.io/crates/v/penrose)](https://crates.io/crates/penrose)

Not ready yet for general use but aiming for a configure in source and recompile
model similar to dwm / Xmonad / Qtile. The code should be well documented and
relatively easy to understand (if not, please let me know!): I'm learning the
XCB API as I go so there are likely multiple places where things are not being
done in the smartest way.

![screenshot](screenshot.png)

### Current project status
If you don't mind a bare bones (_really_ bare bones) window manager then you can
take penrose for a spin using the config in the `example` directory. You will
need to update the keybindings to launch your preferred terminal emulator and
program launcher and may want to adjust the floating window classes to handle
some additional programs.

#### Current functionality
- user defined layout functions (`side_stack` implemented)
- configurable window borders for focused / unfocused
- configurable gaps
- configurable keybindings
- workspaces (including moving clients between workspaces)
- kill focused client
- cycle focus

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
