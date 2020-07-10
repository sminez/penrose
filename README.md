Penrose - a tiling window manager in the style of dwm / xmonad
==============================================================
![Build](https://github.com/sminez/penrose/workflows/Build/badge.svg?branch=master) ![crates.io version](https://img.shields.io/crates/v/penrose)

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


### Current TODO list
- [ ] drag clients through stack
- [ ] track focused monitor with multi-monitor setup
- [ ] move client to monitor
- [ ] single client fullscreen for all layouts (configurable)
- [ ] Run several / hooks for keybindings
  - It'd be nice if it were possible to just give a block of actions to be run
  but I'm not sure how that will work with the `gen_keybindings` macro
- [ ] dwm style bar and systray
  - This one is probably a decent amount of work...I still prefer it to having
  to install and configure something like lemonbar/polybar though and exposing
  the API via xsetroot is a nice touch that I'd like to keep.
  - While the Qtile bar I used to have was my favourite by far, I doubt I'm
  going to be able to reproduce that any time soon starting from scratch!

- [ ] Scratchpads
  - My dwm set up currently has a single scratchpad (a terminal) but I've
  previously has a set up where different bindings triggered scratchpads for
  different programs. Not decided yet on whether I prefer that (hard coded
  scratch pads) or the ability to tag windows as being on a scratch pad and then
  cycling through them a-la i3.

- [ ] Mouse bindings
  - I'm currently grabbing `MOD-{1, 3}` on start up with the intension of using
  that for mouse based move and resize respectively. The xcb events provide
  cursor position info so it should be a case of generating a target region from
  that and applying the resize.
  - Probably should factor out the resize logic so this can be reused then...
  - getting draw-term back (possibly as a build in feature) would be pretty cool
