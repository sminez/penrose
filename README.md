<image width="100px" src="icon.svg" align="left"></image>
penrose - a tiling window manager library
=========================================

[![Build](https://github.com/sminez/penrose/workflows/Build/badge.svg?branch=develop)](https://github.com/sminez/penrose/actions?query=workflow%3ABuild) [![crates.io version](https://img.shields.io/crates/v/penrose)](https://crates.io/crates/penrose)

:warning: Multiple breaking API changes are being made in advance of the 0.2.0
release. Please see commit history at the current time for details. :warning:

`penrose` is a modular library for configuring your own X11 window manager in
Rust. It uses a workspace model as opposed to dwm or awesome style tags and
provides a default backend for interacting with the X server written on top of
the XCB API. The project is very much a work in progress as I try to set up my
ideal, minimal window manager for daily use and in its current state you should
be prepared to see breaking API changes as things stabilise. That said,
`penrose` is now feature complete enough and stable enough to use as your
primary Window Manager, so long as you don't mind a few rough edges! I am aiming
for the code to be well documented and easy to extend. For now, this is my
primary hobby project so updates are frequent: I try to keep crates.io up to
date when new major features are completed but you can follow the `develop`
branch in github for the latest changes. (Please note that `develop` is as it
sounds and that stability is not guaranteed in any way.)

![screenshot](screenshot.png)

### Current project status

While the project is still in its early stages, please expect there to be
multiple breaking changes as the public API stabilises. The example config files
will always be kept up to date so please refer to them for updating to newer
versions published to cargo. I try, where possible, to ensure that all
functionality is showcased in at least one of the examples but you should take a
look at the docs on docs.rs for the full public API if you want to check out all
of the available functionality.

I am currently using penrose as [my daily driver](https://github.com/sminez/my-penrose-config)
and actively working on the project: poking around in the guts of various
existing window managers, seeing what I like and what I want to incorporate.
Development may be a little sporadic depending on what my current work / home
commitments are but you can typically expect to see updates every few days.
I am trying to provide some demos and examples on
[youtube](https://www.youtube.com/channel/UC04N-5DxEWH4ioK0bvZmF_Q) as I go,
particularly when major new features are added.

### FAQ

#### How do I use penrose?

Please look at the examples provided in the `examples` directory.

#### Does penrose support Wayland as opposed to X11?

No. Wayland merges the window manager with the compositor which is significantly
more work. Unless you want to do it yourself, Wayland support is "not a thing".

#### Can you add this piece of eye candy to penrose?

No. The core of `penrose` is a fast, minimalist window manager. Window
decorations, animations and screenshot friendly window positioning are not
useful to me in my day to day work and I will not be spending any of my free
time implementing them. If you can find a way of implementing them that does not
modify anything in the `src/core` directory then feel free to raise an issue
outlining your implementation for inclusion in the `contrib` directory.

#### Are you accepting contributions?

If there is a feature that you would like to contribute as an extension to
`penrose` for the `contrib` directory then please raise an issue explaining the
use case, detailing your intended implementation. At the moment I do not have a
concrete set of guidlines for contributers given the small nature of the project
but I do work have a lot of preferences around how I want penrose to work. If
you are wanting to pick up and work on one of the issues currently open please
start a discussion about it on the issue itself before raising a PR: I am
actively working on multiple aspects of the project at once and you may find
things moving underneath you!

#### Will you add feature X from $OTHER_WM?

In all honesty, probably not. At this stage, `penrose` covers everything that I
want / need from my window manager. I am more that happy to accept contributions
for the `contrib` directory that extend the existing functionality but a key
aspect of this project is that I want to understand the code that I am running
on my machine. If the feature in question can not be implemented as an
extension, feel free to raise an issue explaining your use case and why you feel
it is needed but be prepared to write the implementation yourself.

### Getting started

Depending on how much tinkering you want to do, there are several example
`main.rs` files in the `examples` directory that you can use as a starting point
for your configuration. You will need to ensure that you at least have a
terminal and program launcher set up (the defaults are `dmenu` and `st` from
https://suckless.org) otherwise you are going to be unable to spawn programs!

As mentioned above, my personal set up is also hosted on github and runs from
the head of develop as opposed to pinning at a specific released version. This
is not recommended in general as develop is not guaranteed to be stable in any
way (there are also likely to be intermittent breaking API changes as I iterate
on the best way to do things). The aim is to provide an Xmonad style "extend
with your own custom code" experience though obviously at present, penrose has
no where near the same number of out of the box libraries and examples to work
from so you will likely need to port over your favourite Xmonad / other WM
features if you find them missing. If you are happy to do so, please do raise a
PR and I can incorporate your favourite feature into the `contrib` directory.

If you update to a new version from crates.io (or are tracking develop) and you
suddenly get compile errors in your config, please check the documentaton hosted
on [docs.rs](https://docs.rs/penrose) to see if there have been any recent API
changes. I am trying to keep breaking changes to a minimum but at this early
stage in the project there are multiple things in flux as the codebase
stabilises.

### Current functionality
My personal set up has floated between a variety of tiling window managers over
the years, ranging from i3, dwm and Qtile to Xmonad, Wmii and BSPWM. `penrose`
is my attempt to cherry pick the functionality I make use of while also
providing a flexible base to build from should you want to get your hands dirty
with some rust yourself. As a non-exhaustive high-level overview of what is
currently implemented you can take a look at the list below. Alternatively, have
a look through the documentation (particularly the public methods on
`WindowManager`) to see what is available.

#### Implemented
- multi-monitor support
- dynamic layouts
- user defined hooks in response to `WindowManager` / `X` events
- partial EWMH support (active window/desktop, number of desktops, desktop names, window manager name, desktop for client)
- built in layout functions
- user definable layout functions
- hook based status bars
- scratchpads
- dynamic workspace creation
- custom key bindings (able to trigger arbitrary rust code)


### Project Non-goals
#### A config file
Parsing a config file and dynamically switching behaviour on the contents adds a
huge amount of complexity to the code. `penrose` is written as a library
("crate" in the rust lingo) that you use to build your window manager as you see
fit. There is a default set of behaviours that are mostly an opinionated hybrid of
[dwm](https://dwm.suckless.org/) and [i3](https://i3wm.org/) but you are free to
swap out pretty much everything should you wish.
For example, if you prefer `dwm` style tags to workspaces, you should be able to
get that up and running using the hooks system.

#### IPC / relying on external programs
I love acme from plan9 and how easy it is to drive it's state from external
programs (check out my [acme-corp](https://github.com/sminez/acme-corp) tools to
see what I mean) but that comes at the expense of the internal logic becoming
_massively_ more complicated. As few moving parts as possible is ideal. So,
things that are easy to acomplish using the XCB api (key bindings, simple
rendering of a bar etc) are in, full on IPC via an exposed API is out.
That's not to say that making use of external programs is out all together: just
that window manager functionaly is internally implemented rather than relying on
external processes for things like key bindings and window placement.
