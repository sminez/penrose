<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Introduction

Welcome to [Penrose][0]: a modular tiling window manager library for X11 written in Rust.

Unlike most other tiling window managers, `Penrose` is not a binary that you install on your
system. Instead, you use it like a normal dependency in your own crate for writing your own
window manager. Don't worry, the top level API is well documented and a lot of things will
work out of the box, and if you fancy digging deeper you'll find lots of opportunities to
customise things to your liking.

If you are new to Rust it is worthwhile taking a look at the [learning materials][1]
provided by the Rust project to get up to speed on how the language works. (The rest of
this book assumes you are somewhat familiar with the language).

The rest of this book covers the concepts and implementation of `Penrose` at a level of
detail that should allow you to implement your own extensions and custom functionality
on top of the base library. If you just want to skip ahead to a working, minimal window
manager then take a look at the Quickstart section of this book or the [examples][2]
directory of the GitHub repo. (My [personal config][3] is also available to take a look
at if you want to see what something a bit more involved looks like!)

As with all crates on crates.io, the crate level documentation is also available
[on docs.rs][4].

Happy window managing!


  [0]: https://github.com/sminez/penrose
  [1]: https://www.rust-lang.org/learn
  [2]: https://github.com/sminez/penrose/tree/develop/examples
  [3]: https://github.com/sminez/my-penrose-config
  [4]: https://docs.rs/penrose
