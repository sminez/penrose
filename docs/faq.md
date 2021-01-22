<image width="60px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Penrose FAQs
<br>

## How do I install Penrose?
You don't: Penrose is a library that you use to write your own window manager.
Take a look at the [getting started guide][3] for details of how to use Penrose
as a library.
<br><br>

## Where can I view the Penrose source code?

Penrose is developed openly on [GitHub][0] and published to [crates.io][1]
periodically as new features are added. The `develop` branch always has the
latest code and is what I use for running Penrose on my personal laptop. It
is not advised that you pin your use of Penrose to the GitHub `develop` branch
as a typically end user however: breaking changes (and weird and wonderful bugs)
are highly likely. You have been warned!
<br><br>


## How does Penrose differ from other tiling window managers?

Penrose is a tiling window manager _library_ rather than a tiling window
manager. It provides core logic and traits (interfaces) for writing your own
tiling window manager, along with default implementations that can be used out
of the box. That said, you can't **install** Penrose: you need to write your own
Rust [crate][2] that brings in Penrose as a dependency.

The Penrose repository has several up to date examples of what a typical
`main.rs` ends up looking like and there is a guide on how to go from installing
rust to running Penrose as your window manager located [here][3]
<br><br>


## Does Penrose support Wayland as a back end?

Short answer: no.

Long answer: Wayland merges the concept of the window manager with the that of the
compositor, which results in significantly more work (which I'm not planning on
doing given that I'm perfectly happy with X11 as a back end). The internal APIs
of Penrose only expect to be managing window positioning and workspaces (as far
as X is concerned) so while it _may_ be possibly to add Wayland support, it's
not a simple task.
<br><br>


## Where's the eye candy?

Penrose is, first and foremost, designed with simplicity, speed and stability in
mind. This means that the default, out of the box offering is pretty minimal.
I'm a big fan of the [unix philosophy][4] and with that in mind, Penrose largely
restricts its core functionality to managing your windows. Decorations and
animation are not first class citizens but can be added through extensions and
user code if desired.

  [0]: https://github.com/sminez/penrose
  [1]: https://crates.io/crates/penrose
  [2]: https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html
  [3]: https://github.com/sminez/penrose/blob/develop/docs/getting_started.md
  [4]: https://en.wikipedia.org/wiki/Unix_philosophy

