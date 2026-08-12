Key bindings are currently parsed into keycodes by shelling out to `xmodmap
-pke`. This changes them to parse into keysyms, and moves keysym -> keycode
resolution into the x11rb backend where it happens against the server at grab
time.

```rust
// unchanged in configs
let raw_bindings = map! {
    map_keys: |k: &str| k.to_string();

    "M-S-semicolon" => spawn("dmenu_run"),
    "XF86AudioMute" => toggle_audio(),
};

// parse_keybindings_with_xmodmap(raw_bindings) still works, but is deprecated
let bindings = parse_keybindings(raw_bindings)?;
```

## Why

Two problems with keying bindings on keycodes.

**A keysym can live on more than one keycode.** `keycodes_from_xmodmap` returns
`HashMap<String, u8>`, so when a layout puts the same keysym on two keys - or
when the keypad duplicates one - only one of them survives, and a binding
silently does nothing on the other key. xmonad grabs all of them: `mkGrabs`
builds a `Map KeySym [KeyCode]` with `fromListWith (++)`
(`XMonad/Operations.hs`), and this does the same.

**`xmodmap` is a subprocess and an X11 client.** It has to be installed, it
reports the keymap of whatever display it connects to, and
`keycodes_from_xmodmap`'s own doc comment admits it panics if the output is not
what it expects. Parsing is now pure, which also means a config's "do my
bindings parse" test runs without a display - that test previously needed a
running X server to be meaningful.

Keysyms are also the currency every other window system uses. xkbcommon reuses
X11's keysym numbering, so a keymap parsed this way is portable data rather than
something tied to one server's keycode assignment.

## Breaking changes for `Conn` implementors

`Conn::KeyBindingKey` is `KeySym` rather than `KeyCode` for X11 backends:

```rust
pub struct KeySym { pub mask: u16, pub keysym: u32 }
```

`XConn::grab` takes `&[KeySym]`, and backends resolve to whatever their platform
grabs with. `KeyCode` is unchanged and is now an x11rb implementation detail.
`capture_next_key` also gains a `continuations` argument, for the reasons below.

For x11rb that resolution is `GetKeyboardMapping` plus two pure lookups over the
returned table: every keycode carrying a keysym at any level (what to grab), and
every level of a pressed keycode (what a press means). Both are unit tested
against a hand written table rather than a live server.

## `parse_keybindings_with_xmodmap` is deprecated, not removed

It now delegates to `parse_keybindings` and no longer runs `xmodmap`, so
existing configs keep compiling and keep working. `keycodes_from_xmodmap` is
untouched for anyone using it directly.

`parse_keybindings` keeps its name and its job - the bindings that parsed
alongside the errors for those that did not - and loses its `Result` wrapper,
since with no `xmodmap` to run there is nothing left for the call as a whole to
fail at.

## `penrose_keysyms` gains keysym values

`XKeySym::keysym()` returns the X11 keysym value. The numbers were already in
the crate, inside `as_utf8_string`'s match, so this exposes them rather than
adding data.

The crate is also no longer optional, since name -> keysym now lives in
`core::bindings`. The `keysyms` feature is kept as a no-op.

## Known gap: names outside the generated table

`XKeySym` covers about 680 of the ~2300 names in `keysymdef.h` and
`XF86keysym.h`, which was plenty when it only backed the utf8 conversion and is
now the set of names that parse. `xmodmap` would accept anything in the current
keymap, so a binding on a name outside the table - `XF86AudioPause`,
`XF86Sleep`, the `XF86Launch*` keys, any `dead_*` key, `Multi_key` - now fails
to parse where it used to work.

For comparison, xmonad's EZConfig accepts less than this: printable ASCII and
Latin-1 as literal characters, F1-F24, about fifty named specials and a
hand-curated list of ~77 multimedia keys (`XMonad/Prelude.hs`). So the table is
not obviously the wrong size, and regenerating it from the headers is a
mechanical change if this turns out to bite anyone.

`KeyBindings::parse` takes the parse function as a parameter, so a config
needing a name the table lacks can supply its own resolver without waiting for
that.

## Behaviour: both `S-semicolon` and `S-colon` work

`xmodmap -pke` lists every level of a key on one line and
`keycodes_from_xmodmap` maps all of them to that keycode, so today `"S-colon"`
and `"S-semicolon"` are the same binding. Under keysyms they are different keys,
and to keep both spellings working a press is matched against every level of the
keycode it came from, taking the first level that is actually bound.

One consequence: binding *both* spellings to different actions is currently
caught at parse time as a `DuplicateKeyBinding` error, since they produce the
same `KeyCode`. Now they are distinct keys that land on the same physical key,
so the clash is reported as a warning when grabbing instead.

## `capture_next_key` is told which keys would continue the sequence

`capture_next_key` was asked to catch the next key without being told which keys
mattered, which left the backend guessing at what it had caught. It now takes
them:

```rust
fn capture_next_key(&mut self, continuations: &[Self::KeyBindingKey]) -> Result<()>;
```

On X11 that is what makes the level matching above apply mid-sequence as well as
to a leader. `grab` receives the leading keys, which is also the set a press is
resolved against, so before this the rest of a sequence - which arrives through
the keyboard capture rather than through a grab - had nothing to match against
and fell back to level 0. `"M-m S-colon"` matched nothing and abandoned the
sequence where `"M-m S-semicolon"` worked; now both run, the same way both
spellings work for a leader.

It also makes a compositor backed backend possible, which is the reason it is
worth the trait change rather than a private fix. Where X11 grabs the whole
keyboard and is handed every press with its keycode, a compositor which binds
keys on the window manager's behalf sends nothing at all for a key it has no
binding registered for, so the continuations have to be registered for the
duration of the capture. river's `ensure_next_key_eaten` covers only the other
half: it stops the key reaching the focused client and reports that an unbound
key was pressed, without saying which.

A second call before the captured press arrives replaces the expected keys
rather than adding to them, so a backend registering them elsewhere knows to
drop the previous set.

## Behaviour: an unknown key name fails later than it used to

A name that is not in the current keymap used to fail at parse time, because it
was not in the `xmodmap` output. It now parses - the keysym exists whether or
not your keyboard can produce it - and is reported when grabbing finds no
keycode for it. That seems like the right place for it: whether a key exists is
a property of the keyboard, not of the config, and the same config should parse
on a machine whose layout differs.

## Relationship to #344

Sits on top of #344. The keysym half is independent of it and touches the same
code: #344 added `KeyBindings::parse`, which takes the parse function as a
parameter, so the binding syntax, sequence grouping and error accumulation are
all unaffected - `KeySym::parse` just replaces the closure that was consulting
the `xmodmap` map.

The `capture_next_key` change above is a direct follow-up to #344, which
introduced that method, and closes the level matching gap that PR left open for
sequence continuations.
