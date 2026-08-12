This adds support for bindings which run after a *sequence* of key presses, as requested in #260. These are written like this:

```rust
let raw_bindings = map! {
    map_keys: |k: &str| k.to_string();

    "M-m M-l" => spotify_like(),
    "M-m M-n" => spotify_next(),
    "M-m M-m" => spotify_toggle_play(),
};
```

This is the same style as xmonad's [`XMonad.Util.EZConfig`](https://hackage-content.haskell.org/package/xmonad-contrib-0.18.2/docs/XMonad-Util-EZConfig.html).

## One slightly breaking interface change

`KeyBindings<C>` changes from a type alias for `HashMap` to a struct. Seems
unlikely to affect anyone's configs - doesn't appear to in any config I found on GitHub.

## Breaking changes for `Conn` implementors

`capture_next_key` and `cancel_capture_next_key` are **required** `Conn`
methods, and backends should now call `dispatch_key`. These are needed for
sequence abort - without it a mistyped sequence would sit armed until some
matching key is pressed.

These are supported by [river](https://codeberg.org/river/river), and so supporting this on Wayland will be straightforward.

## Use of `extract_if` requires rust 1.88

For context it was released at the end of June.

## Keybinding parse errors are accumulated rather than stopping at the first

`parse_keybindings_with_xmodmap` still fails if anything failed, now listing all of them. To me it would make sense to deprecate this.

`parse_keybindings` is new, and returns the bindings that parsed alongside the errors for
those that did not. One typo shouldn't necessarily stop the window manager from starting, so
the caller picks: `into_result()` to fail if anything did not parse, `log_err()` to log those
and run with the rest.

## Behavior change: duplicate bindings are now errors

Before this, if the user bound both `M-S-j` and `S-M-j`, one of them would win non-deterministically based on HashMap order. Now this is an error.

## Open question: Documentation

I'm unsure whether I should just update an example or what.
