<image width="100px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
penrose - a tiling window manager library
=========================================

[![Build](https://github.com/sminez/penrose/workflows/Build/badge.svg?branch=develop)](https://github.com/sminez/penrose/actions?query=workflow%3ABuild) [![crates.io version](https://img.shields.io/crates/v/penrose)](https://crates.io/crates/penrose) [![docs.rs](https://img.shields.io/docsrs/penrose?logo=rust)](https://docs.rs/penrose) [![Book Build](https://github.com/sminez/penrose/actions/workflows/book.yml/badge.svg)](https://github.com/sminez/penrose/actions/workflows/book.yml)

### `Penrose` is a modular library for configuring your own X11 window manager in Rust.

This means that, unlike most other tiling window managers, `Penrose` is not a
binary that you install on your system. Instead, you use it like a normal
dependency in your own crate for writing your own window manager. Don't worry,
the top level API is well documented and a lot of things will work out of the
box, and if you fancy digging deeper you'll find lots of opportunities to
customise things to your liking.

![screenshot](https://raw.githubusercontent.com/sminez/penrose/develop/screenshot.png)

### tl;dr - getting started

The docs for penrose are written using mdBook and published to GitHub Pages [here][0].
They cover some more general concepts about how to get up and running as opposed to the
[crate docs][1] on docs.rs which are more aimed at covering the APIs themselves.

> The current development version of the docs can be found [here][2].

If you want to have a look at how it all comes together then the [examples][3] directory
of this repo has several different starting points for you to begin with and my personal
set up can be found [here][4]. (You almost certainly _don't_ want to use my set up in
full but it should serve as a good reference for what a real use case looks like!)

Join us on discord [here](https://discord.gg/jtFsg2K3Fw)

<br>


### Project Goals

#### Understandable code

`Penrose` was born out of my failed attempts to refactor the [dwm][5] codebase into
something that I could more easily understand and hack on. While I very much
admire and aim for minimalism in code, I personally feel that it becomes a problem
when your code base starts playing code golf to keep things short for the sake of it.

I certainly won't claim that `Penrose` has the cleanest code base you've ever seen,
but it should be readable in addition to being fast.


#### Simple to configure

I've also tried my hand at [Xmonad][6] in the past. I love the setups people can
achieve with it ([this one][7] is a personal favourite), but doing everything in
Haskell was a deal breaker for me. I'm sure many people will say the same thing
about Rust, but then at least I'm giving you some more options!

With `Penrose`, a simple window manager can be written in about 5 minutes and under
100 lines of code. It will be pretty minimal, but each additional feature (such as a
status bar, scratch-pads, custom layouts, dynamic menus...) can be added in as
little as a single line. If the functionality you want isn't available however,
that leads us on to...


#### Easy to extend

[dwm][5] patches, [qtile][8] lazy APIs, [i3][9] IPC configuration; all of these
definitely work but they are not what I'm after. Again, the [Xmonad][7] model of
companion libraries that you bring in and use as part of writing your own window
manager has always felt like the right model for me for extending the window
manager.

`Penrose` provides a set of traits and APIs for extending the minimal core library
that is provided out of the box. By default, you essentially get an event loop and
a nice clean split between the "pure" state manipulation APIs for managing your
windows and a "diff and render" layer that interacts with the X server. There are
enough built-in pieces to show how everything works and then some more interesting
/ useful code available in the [extensions][10] module.

<br>

### Project Non-goals

#### An external config file

Parsing a config file and dynamically switching behaviour on the contents adds a
large amount of complexity to the code, not to mention the need for _validating_
the config file! By default, `Penrose` is configured statically in your **main.rs**
and compiled each time you want to make changes (similar to [Xmonad][7] and [dwm][6]).
There is no built-in support for hot reloading of changes or wrappers around the
main window manager process.

That said, the extensibility of `Penrose` means that you are definitely able to define
your own config file format and parse that as part of your startup, if that is something
you want.

The choice is yours!


#### IPC / relying on external programs for core functionality

There are several places where `Penrose` makes use of external programs for
utility functionality (reading the user's key-map or spawning a program launcher
for example), but core window manager functionality is contained in the pure state
data structures. This makes it a lot simpler to maintain the codebase and (importantly)
provide a nice API to work with for extending the behaviour of your window manager.

As you might expect, you can definitely write your own extensions that provide
some sort of IPC or client/server style mechanism if you want to mirror the
kinds of things possible in other window managers such as `i3` or `bspwm`, but
that is not going to be supported in the core of the library itself.


  [0]: https://sminez.github.io/penrose
  [1]: https://docs.rs/penrose
  [2]: https://sminez.github.io/penrose/rustdoc/penrose
  [3]: https://github.com/sminez/penrose/tree/develop/examples
  [4]: https://github.com/sminez/my-penrose-config
  [5]: https://dwm.suckless.org/
  [6]: https://xmonad.org/
  [7]: https://www.youtube.com/watch?v=70IxjLEmomg
  [8]: http://www.qtile.org/
  [9]: https://i3wm.org/
  [10]: src/extensions/
