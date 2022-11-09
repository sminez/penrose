# EWMH

Support for [EWMH][0] in penrose is provided (surprisingly enough) via the [ewmh][1] module
in `extensions`. This provides minimal support for floating windows and setting the appropriate
properties for interaction with things like external status bars (`polybar` for example).

The [add_ewmh_hooks][2] function can be applied to an existing `Config` in order to set up the
required hooks for adding this support.


  [0]: https://specifications.freedesktop.org/wm-spec/latest/
  [1]: https://sminez.github.io/penrose/rustdoc/penrose/extensions/hooks/ewmh/index.html
  [2]: https://sminez.github.io/penrose/rustdoc/penrose/extensions/hooks/ewmh/fn.add_ewmh_hooks.html
