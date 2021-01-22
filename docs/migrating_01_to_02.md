<image width="60px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Migrating your config from 0.1 to 0.2
<br>

The following is a high level overview of the main user facing changes to the
Penrose API when updating your `0.1.X` config to `0.2.0`. For the most part,
the compiler should guide you through the changes but it is worthwhile reading
through the changes that follow in order to guide you through what you need to
do.

There are multiple other internal changes, refactors and additions that can all
be found in the [documentation][0], but these are the main breaking changes that
are likely to leave you scratching your head if you just trying bumping the
minor verison!

<br>

## Errors

Penrose now uses the [thiserror][1] crate for defining `Error` values instead
of [anyhow][2]. This allows for better error handling both inside of Penrose
itself and within user code as well. There is a top level [PenroseError][3] type
that is returned by most functions and methods in the crate, but there are also
specific [DrawError][4] and [XcbError][5] types for those respective modules.

<br>

## Results

Previously, keybindings were attached to functions matching the `FireAndForget`
function signature which simply ran your code and expected you to handle all
failures internally. This type has been renamed to [KeyEventHandler][6] and now
returns a `penrose::Result`. All corresponding public methods on `WindowManager`
have been updated to match this signature and now propagate errors back to
callers when they occur.

In some cases this has also resulted in modified return types of these methods:
now returning an `Error` in cases where previously you would have received a
default value.

<br>

## WindowManager is now generic over XConn

In `Penrose 0.1`, the `WindowManager` struct took a `Box<dyn XConn>` which was
used to communicate with the X server. This was then inaccessible to user code
once the manager was created, meaning that there was no way for user code to
interact directly with the X server. In `0.2`, the [WindowManager][7] is generic
over the [XConn][8] implementation that is provided (non-boxed) and it is now
possible to write specific `impl` blocks for a `WindowManager` using your
implementation of `XConn` should you wish.

<b>NOTE</b>: Interacting with the X server directly without going through the
`WindowManager` can (depending on what you do) lead to invalid state in the
`WindowManager`. Please be careful with how you make use of this!

One knock on effect of this is that types and traits that take references to a
`WindowManager` are now generic as well. This means that you will have to add
generic types to any custom hooks, widgets and keybinding functions you have
written. (It also means that you can customise the behaviour of these depending
on what `XConn` is being used, if that is of any interest.)

<br>

## Config is now immutable

The `Config` struct is now immutable by default and requires that you follow a
[builder pattern][9]. This provides more flexability in how you build up your
config while also ensuring that you are not able to inadvertantly create invalid
config which is then only caught at runtime.

  [0]: https://docs.rs/penrose/0.2.0/penrose/index.html
  [1]: https://crates.io/crates/thiserror
  [2]: https://crates.io/crates/anyhow
  [3]: https://docs.rs/penrose/0.2.0/penrose/enum.PenroseError.html
  [4]: https://docs.rs/penrose/0.2.0/penrose/draw/enum.DrawError.html
  [5]: https://docs.rs/penrose/0.2.0/penrose/xcb/enum.XcbError.html
  [6]: https://docs.rs/penrose/0.2.0/penrose/core/bindings/type.KeyEventHandler.html
  [7]: https://docs.rs/penrose/0.2.0/penrose/struct.WindowManager.html
  [8]: https://docs.rs/penrose/0.2.0/penrose/core/xconnection/trait.XConn.html
  [9]: https://docs.rs/penrose/0.2.0/penrose/core/config/struct.ConfigBuilder.html

