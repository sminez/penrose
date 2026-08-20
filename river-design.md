# Penrose on river: design notes

A second backend for the vendored penrose (`vendor/penrose`), behind a feature
flag, targeting the [river](https://codeberg.org/river/river) Wayland
compositor's `river-window-management-v1` protocol. Companion to
[`design.md`](../../design.md), which is the X11 port of `~/env/src/xmonad.hs`
and whose section numbers are referenced as "X11 §n". Prior art is surveyed in
[`orilla-report.md`](../../orilla-report.md); the two implementations it
compares — `vendor/orilla` (Rust, river) and `~/env/xmonad-river` (Haskell,
river) — are cited below rather than re-explained.

This file lives in the vendored fork, so source references are paths within it
and name symbols rather than line numbers, which rot. Paths to the sibling
crates are relative to the fork root: `../orilla/…`. Protocol facts checked
against the vendored copy in `src/river/protocol/` (window management v5,
xkb bindings v3, layer shell v1 — see §3) and against a river checkout at
`~/oss/river`, which is cited when the XML alone does not settle a question.

**Why this is possible at all.** River's master branch implements no window
management: position, size, focus, keybindings and decorations are all deferred
to a separate process. So layouts stay ordinary pure code, `M-q` becomes
recoverable (river hot-swaps window managers without restarting clients), and
X11 keysym numbers and modifier masks carry over unchanged — `mod4Mask` is still
64, `XK_Return` still `0xff0d`. Nothing else in Wayland offers this.

**Scope: the window manager draws nothing.** It creates no surfaces of its own —
no prompts, no status bar, no decorations. It binds `river_layer_shell_v1`
(mandatory: without it river closes every layer surface on sight, so no bars, no
notifications, no menus) and consumes what that global reports, but every pixel
on screen belongs to some other client. Bars and menus are separate programs
(§8, §5), which is what design.md §9 already concluded for X11.

That is a scope decision, and it pays for itself immediately. Drawing is what
forces `wl_shm`, and a `wl_shm_pool` comes from a request carrying a file
descriptor — `SCM_RIGHTS` is the one part of the Wayland wire format that
genuinely needs C, and the sole reason `~/env/xmonad-river` carries a `cbits/`
directory. Not drawing means the river build needs no fd passing, no buffer
management, no cairo/pango, and — since §5 keeps name → keysym in Rust — no C at
all.

---

## 1. Feature flags

```toml
[features]
default = ["x11rb"]
x11rb     = ["dep:x11rb"]
x11rb-xcb = ["x11rb", "x11rb/allow-unsafe-code"]
river     = ["dep:wayland-client", "dep:wayland-scanner"]
```

The X11 feature keeps the name it already has. `x11rb` is currently the implicit
feature of an optional dependency, so making it explicit is the whole change, and
`x11rb-xcb` keeps working unaltered. Renaming it to `x11` would read better
against `river` and would break every config that names it, which is a bad trade
for a cosmetic gain — the module it gates is `src/x11rb/` anyway.

Additive, both may be on, neither is required: `--no-default-features
--features river` builds a crate with no X11 anywhere. `penrose_keysyms` is
unconditional rather than X11-gated, because §5 puts name → keysym in `core` and
that crate is the table. Every dependency here is pure Rust, so neither build
needs a C toolchain — see Scope, and §5 for why that constrains the choice.

The seam already exists. `Conn` (`src/core/conn.rs`) is documented as "a
platform agnostic backing connection", `XConn` (`src/x/mod.rs`) is a
convenience layer over it with a blanket `impl<X> Conn for X` in the same file,
and `Conn::KeyBindingKey` was added precisely so a backend
can key bindings on something other than an X11 keycode. `penrosx` (macOS) is
the existing proof, out of tree. So `src/x/` and `src/x11rb/` gate wholesale
behind `x11rb`, `src/river/` appears behind `river`, and `core`/`pure`/`builtin`
stay unconditional.

`src/core/bindings.rs` was the awkward part of this and is already done: the
keysym work (§5) removed its dependency on `x::XConn` and on running `xmodmap`,
leaving `KeySym`, `KeyCode`, `ModifierKey`, `MouseState`, `MouseButton`,
`KeyBindings` and `dispatch_key` all backend-agnostic. `TryFrom<XKeySym> for
KeyPress` stays put rather than moving to `src/x/`, because `penrose_keysyms` is
unconditional now and river names keys the same way.

Two things still violate the seam:

- Seven extension modules name `XConn`. Five are X11 by substance and gate behind
  `x11rb`: `hooks/ewmh.rs`, `hooks/window_swallowing.rs` (matches on `XEvent`),
  `hooks/startup.rs` (interns atoms), the fullscreen/`WM_CLASS` half of
  `actions/mod.rs`, and `actions/dynamic_select.rs` — whose bound looks
  incidental and is not, see §2. Two are X11 by *bound only* and relax
  `X: XConn` to `C: Conn`: `hooks/default_workspaces.rs`, `util/debug.rs`.
- `crates/penrose_ui` is X11 + Xft throughout and is simply not built under
  `river` (§8).

This is a change worth offering upstream: it is the work that makes the
"platform agnostic" claim in `conn.rs` true, and it costs the X11 build no
behaviour, no dependencies and — now that the bindings half has landed — no
churn. Everything left is a bound relaxing or a `cfg` on a module an X11 build
keeps by default, so no existing config changes a line.

## 2. The one hard constraint

River defines two disjoint categories of state and forbids touching either
outside a sequence:

- **manage** (`manage_start` event → … → `manage_finish` request): anything that
  changes what the compositor says to a window — dimensions, focus, fullscreen,
  keyboard and pointer bindings, key eating, close, pointer warp.
- **render** (`render_start` → … → `render_finish`): anything that only changes
  what is drawn — positions, stacking order, borders, hide/show, clip boxes.
  Render state may also be set during a manage sequence, applying at the next
  `render_finish`.

The line between them is not where penrose's API puts it, and the table at the
end of this section is the mapping in full. The two that matter most: a window's
**size is manage state and its position is render state**, so a single
`position_client(id, rect)` feeds both halves; and **hide/show is render**, not
manage, which is what makes `StackSet`'s hidden workspaces a render-only
concern (§6).

Modifying either outside a sequence is a protocol error, which disconnects the
window manager. A manage sequence is always followed by at least one render
sequence; render sequences repeat on their own when a window resizes itself.
`manage_dirty` asks for a sequence the compositor does not know is needed.

Penrose does the opposite. `Conn::modify_and_refresh` runs user code and then
issues `restack`, `position_client`, `show_client`, `hide_client`,
`focus_client` and `set_client_border_color` immediately, whenever a binding
happens to fire.

**Resolution: `RiverConn` buffers, and answers sequences with no user code in
the path.** Every mutating `Conn` method writes into a `Plan` instead of sending
a request. `flush()` writes the queued Wayland requests and, if the plan changed,
sends `manage_dirty`. `next_event()` pumps the Wayland queue; when `manage_start`
or `render_start` arrives, the conn transmits the relevant half of the plan and
finishes the sequence, then keeps pumping. Penrose's run loop
(`WindowManager::run`, `src/core/mod.rs`) never sees the sequence events at all.

### A sequence is answered by a thread that runs no user code

The tempting version of the above is to answer a sequence from inside the
dispatch that received it. That is wrong, and the reason is that river sends the
events which *cause* a sequence in the same batch as the `manage_start` itself:
`river_xkb_binding_v1.pressed`, `ate_unbound_key`, `modifiers_update` and the
layer shell's `focus_exclusive` are each documented as "followed by a
`manage_start` event", and a new `window` event arrives the same way. Answering
from inside the dispatch means the plan transmitted is the one from *before*
penrose handled the event that triggered the sequence, and everything the handler
decided lands one sequence late. That is exactly the ~100ms wrong-binding window
§5 wants not to have, and it is what would make §9's "a manage hook runs before
the window is shown" false.

The obvious fix is to answer the sequence when penrose has handled everything,
which for a single-threaded loop means at the top of the next `next_event()`.
That is correct and it is what this backend did first, and it is also **wrong for
a reason that only shows up in a real config**: river holds input processing while
a manage sequence is open, so a handler that waits on anything holds the
compositor rather than merely the window manager. See "handlers block, and that
is fine" below.

So: **the connection lives on a thread of its own which never runs user code**,
and answers a sequence from the last plan the worker published, waiting a bounded
few milliseconds for a fresher one first. A handler that returns promptly still
lands its decisions in the sequence its own key press provoked, so nothing above
is given up in the ordinary case; one that blocks degrades to X11's failure mode
instead of deadlocking the session. This is `~/env/xmonad-river`'s split, and its
`DESIGN.md` is worth reading before touching any of it.

The split forces one thing that is easy to miss: **input routing is loop state.**
Which bindings are enabled, and whether a capture is open, have to be coherent
with the key press river matched against them, and `ate_unbound_key` arrives on
the loop. So the binding objects, the enabled set and the capture all live there,
and the worker says only "these are the bindings" and "eat the next key, these
would continue it" — as ops. Without that, the `ate_unbound_key` callback reads a
slot the worker is concurrently writing.

Three consequences follow, and all three are xmonad-river's, learned the
expensive way:

- **The plan is a total restatement.** A sequence can start at any moment — a
  new window needs no binding — so the plan must always be complete and always
  safe to re-send. Penrose's refresh is partly diff-based
  (`set_window_visibility`, `set_window_props`), so the conn *accumulates* those
  diffs into its own maps and restates the whole map each sequence.
- **One-shot effects need a separate queue.** Re-sending a position is free;
  re-sending a `close` kills a second window. `kill_client`, `warp_pointer`,
  `grab` and the capture go into a drained `Vec<Op>`, not the plan. All of them
  are manage state, so there is one queue and it drains in the manage sequence.
- **Liveness filtering happens at transmit time.** The plan can name a window
  river has since closed, and every stale reference — position, border,
  visibility, stacking, focus, and each op — is a protocol error. One guard, in
  one place, over the live window map.

The plan is therefore two structs, not one, because the halves are transmitted
by different events and a field in the wrong half is a protocol error:

```rust
struct ManagePlan {
    dimensions:  HashMap<WinId, (u32, u32)>,  // propose_dimensions
    focus:       Option<WinId>,               // None -> river_seat_v1.clear_focus
    fullscreen:  HashSet<WinId>,
    enabled:     HashSet<KeySym>,           // river_xkb_binding_v1.enable / .disable
    capture_key: bool,                        // ensure_next_key_eaten (§5)
    layer_default: Option<OutputId>,          // which output unplaced layer surfaces land on
    ops:         Vec<Op>,                     // drained, not restated
}
struct RenderPlan {
    order:     Vec<WinId>,          // bottom-to-top, from restack -> place_above chain
    positions: HashMap<WinId, Point>,
    borders:   HashMap<WinId, (u32, Color)>,
    visible:   HashSet<WinId>,      // hide / show
}
enum Op { Close(WinId), WarpPointer(WinId, i16, i16) }
```

`ManagePlan::fullscreen` is `inform_fullscreen`, not `fullscreen`. The protocol
splits the two: `fullscreen` is geometry — river fills a named output and takes
position and size away from the window manager — and `inform_fullscreen` is the
state in the window's own configure, which is what the window itself reads. Only
the second is sent. A tiling window manager has a tile to offer and nothing
larger, so a window that asked for fullscreen is told it has it and presents
that way inside the bounds penrose gave it, rather than covering the output,
the bar and whatever else is on that workspace.

Sending neither is the trap. A window has already entered its own fullscreen by
the time it asks, so ignoring the request looks like it worked — until the next
configure the window gets for any other reason, which it reconciles against a
state that never said fullscreen. Losing focus is such a configure, and so is
being hidden by a workspace switch: that is "the video un-fullscreens when I
switch away", with everything else correct.

A window's own `fullscreen_requested` is therefore answered by the backend
rather than left to the config, which is where X11 leaves it: there the state
is an EWMH property and opt-in with the rest of EWMH, and here it arrives on
the protocol the backend already speaks, with nothing else able to answer it.
`set_capabilities` says the same thing to the window and has to agree with it,
which it cannot if the two are decided in different places. A config takes the
decision back the ordinary way, with an event hook that returns `false` for the
event.

Filling the screen is a separate decision, and not one river forces either way.
Penrose fullscreens on X11 by floating the client at its screen's rect and
dropping its border, which is a pure-layer move available here too and owes
nothing to river's `fullscreen` request. It is deliberately not made: a window
that asked for fullscreen while tiled is still one of several on its workspace,
and the tile it has is the honest answer. So `RiverConn::set_fullscreen` and
X11's `toggle_fullscreen` are not one action under two names — the X11 one
covers the screen, this one changes what the window is told and nothing else.

`position_client(id, rect)` splits: `rect.wh` into `ManagePlan::dimensions`,
`rect.xy` into `RenderPlan::positions`. So a layout change is not atomic on
screen — the size lands at `manage_finish` and the position at the following
`render_finish` — which is the protocol's design, not an artifact of buffering:
river needs the window to have acked its new size before it can draw a perfect
frame.

### `Conn` method → river request

`Conn`'s methods are listed in the order they appear in `src/core/conn.rs`.
"Half" is the sequence a request may be made in; `—` marks conn-side state or a
method with no protocol counterpart.

| `Conn` method | river request / event | half |
|---|---|---|
| `root` | — (sentinel `WinId(0)`; only `set_focus`'s fallback reads it, as `clear_focus`) | — |
| `capture_next_key` | `river_xkb_bindings_seat_v1.ensure_next_key_eaten` | manage |
| `cancel_capture_next_key` | `river_xkb_bindings_seat_v1.cancel_ensure_next_key_eaten` | manage |
| `grab` | `river_xkb_bindings_v1.get_xkb_binding`, `river_seat_v1.get_pointer_binding` (any time) then `enable` / `disable` | manage |
| `existing_clients` | `river_window_manager_v1.window` events, all sent before the first `manage_start` | — |
| `screen_details` | `river_output_v1.position` + `.dimensions`, narrowed by `river_layer_shell_output_v1.non_exclusive_area` | — |
| `cursor_position` | `river_seat_v1.pointer_position` (v2), sent in every manage sequence in which it changed | — |
| `warp_pointer` | `river_seat_v1.pointer_warp` (v3) | manage |
| `position_client` | `river_window_v1.propose_dimensions` (w, h) **and** `river_node_v1.set_position` (x, y) | manage **+** render |
| `show_client` / `hide_client` | `river_window_v1.show` / `.hide` | render |
| `withdraw_client` | — (X11 `WM_STATE`; river sends `closed` and the objects are destroyed) | — |
| `kill_client` | `river_window_v1.close` | manage |
| `focus_client` | `river_seat_v1.focus_window`, or `.clear_focus` | manage |
| `client_geometry` | `river_window_v1.dimensions` event plus the planned position | — |
| `client_title` | `river_window_v1.title` event | — |
| `client_pid` | `river_window_v1.unreliable_pid` event (v2) | — |
| `client_should_float` / `client_should_be_managed` | `river_window_v1.app_id` event | — |
| `client_is_fullscreen` | `fullscreen_requested` / `exit_fullscreen_requested` events; `inform_fullscreen` / `inform_not_fullscreen` requests (not `fullscreen`, see above) | manage |
| `client_transient_parent` | `river_window_v1.parent` event | — |
| `set_client_border_color` | `river_window_v1.set_borders` | render |
| `set_initial_properties` | `river_window_v1.use_ssd`, `.set_tiled`, `.set_capabilities` | manage |
| `restack` | `river_node_v1.place_above` chain over the nodes from `river_window_v1.get_node` | render |
| `flush` | write, plus `river_window_manager_v1.manage_dirty` if the manage half changed | — |

`Conn::flush` returns `()`, with nowhere to report a failed write, and that is
fine here rather than something to fix upstream. Wayland requests have no
replies, so the only failure a write can have is a fatal one; a fatal protocol
error arrives as `wl_display.error` and makes every subsequent read fail, so
`next_event()` is already a faithful error channel and a changed signature would
report the same death one call earlier. `wayland-client` buffers internally, so
there is no partial-write case either. `RiverConn` stores the error and returns
it from the next `next_event()`.

What that leaves worth specifying is the death path, since a protocol error is
the likely outcome of every bug in §2: log the interface, object and opcode from
the `wl_display.error`, exit non-zero, and let the supervisor restart through §7.

**Clients do survive an unclean disconnect — confirmed in river's source**, which
matters because the hot swap is an orderly `stop`/`finished` and a crash is not.
`WindowManager.handleDestroy` is the path taken whenever the window manager
object goes away, for any reason, and it calls `makeInert` on every window,
output and seat and then finishes whatever sequence was in flight;
`Window.makeInert` sends `closed` to the departing window manager and detaches
the protocol object without touching the toplevel. `stop` is literally that
function plus a `finished` event. So a crash costs the session its layout and
nothing else.

Getting the non-zero exit is more awkward than it sounds and the reason is worth
recording: `WindowManager::run` consumes the conn *and* returns `Ok` regardless —
its loop hands any error to `handle_error`, which logs it and calls straight back
in — so neither the return value nor the conn can carry the news. Hence
`RiverConn::fatal_watch`, a handle taken before `run` which outlives it, and
which reports only real failures: a hot swap and a compositor shutdown are
orderly and report nothing.

Two things the table makes visible that prose kept hiding. `get_node` is a
protocol error if called twice for one window, so nodes are created once when
the window is first seen and stored beside the `WinId` — the render plan names
nodes, not windows. And every `Conn` method that returns `Result` but maps to a
Wayland *request* can only fail by disconnecting, since requests have no replies;
the only methods that can meaningfully fail are the ones reading conn-side state.

### Handlers block, and that is fine

A blocking handler under X11 is a frozen window manager on a live desktop. Under
river it deadlocks the session, by a specific route:

> `river_layer_shell_seat_v1.focus_exclusive`: A layer shell surface will be
> given exclusive keyboard focus **at the end of the manage sequence in which
> this event is sent.** […] This event will be followed by a `manage_start`
> event.

`DMenu::build_menu` (`src/extensions/util/dmenu.rs`) spawns a menu and blocks on
`read_to_string` of its stdout — for as long as a person takes to choose. Under
Wayland that menu is a layer surface asking for exclusive keyboard focus, so if
the handler's own thread is the one that owes river a `manage_finish`: handler
blocks → `manage_start` unanswered → menu never gets the keyboard → user cannot
type → handler never returns. Under X11 this has never bitten because dmenu is
override-redirect and grabs the keyboard itself, with no window manager in the
path.

(Layer surfaces *map* without per-surface cooperation — binding
`river_layer_shell_v1` once at startup is the whole requirement. It is focus
specifically that is gated on `manage_finish`.)

**This section used to say a handler may not block, as a contract rather than a
mechanism.** The argument was that penrose has essentially one blocking helper
and one config to keep honest, where `~/env/xmonad-river` had to keep running an
ecosystem built on blocking actions. The first half is true of the library and
false of everything else. Counted in the config this fork exists for:

| blocks on | where |
|---|---|
| a person | the `M-x` action menu, two note prompts, byzanz arguments — all `menu::select`/`menu::prompt` over rofi |
| a subprocess, briefly | `amixer`, `timeout … xclip` |
| a subprocess, for tens of seconds | `rebuild-penrose.sh` (a cargo build), a `gist` upload, the byzanz capture |

Nine handler paths, four of them waiting on a person, in a config whose own
`menu.rs` says the blocking is deliberate. Penrose adds four more of its own
(`DMenu::build_menu`, `DMenu::run`, `spawn_for_output`, `util::notify`). A rule
broken nine times before it is even ported is not a rule, and the failure mode
for breaking it is not a slow window manager but a session that has to be killed
from a TTY.

**So the connection gets a thread of its own** (see "a sequence is answered by a
thread that runs no user code"), user code keeps blocking, and the failure mode
goes back to being X11's: window management stops until the handler returns and
everything else carries on. `tests/headless-menu.sh` is that case end to end
against a real `fuzzel` — the handler blocks on it, the keyboard types a choice
into it, and the handler gets the choice.

What it costs is a bounded wait. The loop waits a few milliseconds for the worker
to publish before answering, and longer after a key press, since arming a key
sequence has to be atomic with the press that opened it (§5). Fast handlers
therefore keep the guarantee; a handler that overruns the wait loses it for one
round trip, which is xmonad-river's ~100ms wrong-binding window arriving here for
the same reason. It is a strong preference, not a guarantee, and an unbounded
wait is the deadlock this design exists to remove.

Thread *per action* is not the answer, for the record: handlers take
`&mut State<C>` and penrose serializes actions the way xmonad does, so concurrent
handlers would interleave focus and layout. Wrapping `State` in a mutex moves the
freeze to the lock and adds a race. One worker, in order, exactly as before.

**`DMenu` itself stays X11-only, but for its own reasons now.**
`dmenu_focus_client`, `dmenu_focus_tag` and `launch_dmenu` keep their `X: XConn`
bound — a type bound rather than a `cfg`, because features are additive and both
backends may be on, so gating on `river` would delete `DMenu` from a build that
wants it for its X11 half. The bound is honest on its own terms: `DMenuKind` is
`Suckless | Rust`, both X11 dmenus invoked with dmenu's flags. A river config
wants `fuzzel`, which is a new `DMenuKind` with different arguments — a different
call, not the same one on a different backend.

**Two things are worth saying out loud in the log**, because a rule nobody
checks is a rule that gets broken and a pause nobody explains is a bug report
with no evidence in it. When the loop gives up waiting and answers from the plan
it already has, that is logged at debug with the event still being handled: it
means the handler's decisions land a sequence late, and it is true of every
sequence while a menu is open, so it is not a fault and not a warning. When the
worker has been behind for **ten seconds**, that is a warning, once per episode,
naming the event. Ten rather than one: a person choosing from a menu takes a few
seconds and that is working as intended, where ten seconds is a build or a wedge,
and either way window management has stopped for the duration.

There is also a compositor-side deadline waiting to exist. The protocol defines
`river_window_manager_v1.error.unresponsive`, "window manager unresponsive",
which river does not currently send — `grep unresponsive` over the Zig source at
`~/oss/river` finds nothing, and the only timer in `WindowManager.zig` is a
100ms wait for tracked *window* configures, not for us. If that changes, the loop
is what keeps answering, so a slow handler stays survivable.

## 3. Protocol bindings

`wayland-client` + `wayland-scanner`, expanding the XML with
`generate_interfaces!` / `generate_client_code!` at compile time — exactly
`vendor/orilla`'s `protocol.rs`, which is 118 lines and almost all
module-nesting boilerplate for cross-protocol type references. No libwayland, no
build script, no generated code in the tree.

The XML lives in `src/river/protocol/` — `river-window-management-v1.xml`,
`river-xkb-bindings-v1.xml`, `river-layer-shell-v1.xml` — and the macros take
paths relative to `CARGO_MANIFEST_DIR`, so they point straight at it. Plain files
in the tree: `grep` works, `cargo package` picks them up under the existing
`include = ["src/**/*"]`, and a fresh clone builds with nothing to fetch.

**Copied from a recent river commit, with the commit recorded** in a `SOURCE`
file beside the XML: `bfab9ea` on `main`, 2026-08-07, giving
`river_window_manager_v1` version 5, `river_xkb_bindings_v1` version 3 and
`river_layer_shell_v1` version 1. Those three files are byte-identical to
`~/env/xmonad-river/protocol/`, so everything xmonad-river established by
reading them carries over unaltered. Not from orilla: its copy is
`river_window_management_v1` version 3, which has no `identifier` event at all,
so §7 is impossible against it.

These protocols are unstable and not yet tagged, so a copy goes stale silently —
which is what the staleness check under Future work is for.

Keep a river checkout around regardless, because **the Zig source answers what
the protocol docs do not.** That `river_window_v1.identifier` is stable across a
window manager restart, and that it arrives before the first `manage_start`, are
both §7's foundation and neither is written down in the XML; xmonad-river settled
them by reading `Window.zig` and `WindowManager.manageStart()`.

Three globals to bind: `river_window_manager_v1`, `river_xkb_bindings_v1`,
`river_layer_shell_v1`. Mouse bindings need no fourth — `get_pointer_binding`
hangs off the `river_seat_v1` objects the window manager global already hands
out. Input device configuration — `river-libinput-config-v1`,
`river-xkb-config-v1` — stays river's job in its init script, as it is for
orilla.

## 4. Events and identity

`Conn::Event = RiverEvent`, a small enum of the events penrose actually acts on:
`KeyPress(KeySym)`, `MouseEvent`, `WindowOpened(WinId)`, `WindowClosed(WinId)`,
`Title(WinId)`, `AppId(WinId)`, `FullscreenRequested(WinId, bool)`,
`ScreenChange`, `PointerFocus(WinId)`. Everything else — sequence events,
dimensions, node bookkeeping — is consumed inside the conn. `requires_pointer_warp`
returns `false` for `PointerFocus` and true otherwise, mirroring the X11 `Enter`
rule that is what makes `warpMid` free (design.md, "What comes for free").

`WindowOpened` is not sent when river's `window` event arrives but at the end of
the batch, because a window's app id, title and parent each arrive as their own
event and a manage hook wants all of them. `FullscreenRequested` is queued behind
it for a related reason: a window can map already asking for fullscreen, river
sends both in the one batch, and a request that arrived first would name a window
penrose has not managed — which the backend does not answer for, so the request
would be dropped and the window never told.

`WinId` is an internal counter, not a Wayland object id: object ids are recycled
after `wl_display.delete_id`, so reusing them means a stale `WinId` can name a
live window. Two maps in `Conn::State`, plus the `identifier` string stashed
alongside for restart.

`root()` has no referent under river. Return `WinId(0)` as a sentinel; the only
use is `set_focus`'s fallback (`src/core/conn.rs`), which becomes
`river_seat_v1.clear_focus`.

`existing_clients` / `manage_existing_clients` work because river sends a
`window` event for every existing window *before* the first `manage_start` —
`WindowManager::manageStart()` iterates them all first. A roundtrip in
`RiverConn::new()` is enough; the existing `manage_without_refresh` path is
unchanged.

## 5. Bindings

River registers bindings compositor-side:
`river_xkb_bindings_v1.get_xkb_binding(seat, keysym, modifiers)` returns an
object that reports `pressed`/`released`. The window manager never sees an
unbound key, so there is no grab, no keymap and no `xmodmap`.

This means `KeyBindingKey` is a keysym, not a keycode. That change is **done**,
and was made on the X11 side first rather than as a river-shaped type, so
`Conn::KeyBindingKey` is now

```rust
pub struct KeySym { pub mask: u16, pub keysym: u32 }
```

on every backend and river needs no key type of its own. The reasons had nothing
to do with river: `keycodes_from_xmodmap` returned `HashMap<String, u8>`, so a
keysym that lives on two keycodes silently bound only one of them (xmonad's
`keysymToKeycodes` returns a list and grabs all of them), and the `xmodmap`
subprocess was an X11 client reporting a keymap that need not be the
compositor's — plus its own doc comment admitted it panicked on unexpected
output. x11rb now resolves keysym → keycodes against the server at grab time via
`GetKeyboardMapping`, and the subprocess is gone. It went first for the same
reason key sequences did: the seam is cheaper to settle with one implementation
in the tree.

The catch is that name → keysym now lives in `core`, which decides the
dependency question below rather than §5 deciding it locally.

`Conn::grab` becomes "create a binding per `KeySym` per seat, enable it", and
`MouseState` maps onto `river_seat_v1.get_pointer_binding` the same way.
`get_xkb_binding` may be called at any time, but `enable` and `disable` are
manage state, so binding *objects* are created once at startup and thereafter
only toggled through the plan (§2). `grab`'s replace semantics become "enable
what is named, disable what is not".

**Name → keysym is `penrose_keysyms`, not `xkbcommon`.** The obvious answer is
`xkb::keysym_from_name`, and it is the wrong one, because the lookup belongs in
`core`: penrose's X11 build is pure Rust today (x11rb, no libxcb), so putting a C
dependency in `core` would hand every existing X11 user a toolchain requirement
they do not have, which is enough on its own to sink the change upstream. It
would also undo Scope's payoff — with the lookup in Rust the river build has *no*
C dependency at all, not one.

The table was already in the tree. `penrose_keysyms::XKeySym` is generated from
`X11/keysymdef.h` and derives strum's `EnumString` over serialize names with the
`XK_`/`XF86XK_` prefix stripped (`BackSpace`, `semicolon`, `XF86AudioPlay`),
which is exactly the spelling `xmodmap -pke` printed and therefore exactly what
existing configs contain. The keysym *numbers* were there too, inside
`as_utf8_string`'s match (`XK_a => 0x0061`, `XF86XK_AudioMute => 0x1008FF12`),
so the generated `fn keysym(&self) -> u32` beside it exposes data rather than
adding it. It does not reuse `as_utf8_string`, which renders those numbers
through `to_le_bytes` and a nonzero filter — meaningless for keysyms above
Latin-1.

Two gaps against `keysym_from_name` remain, both cheap to close if they ever
matter: numeric forms (`0x1008FF11`, `U+1F600`) are a parse fallback of a few
lines, and the generated list is fixed where xmodmap reported whatever was
actually in the keymap — `penrose_keysyms` carries 72 `XF86*` names against
several hundred in `XF86keysym.h`, so an exotic media key could need the
generator re-run. Nothing in this config is affected. Modifier parsing is
unchanged — `ModifierKey`'s mask values are already river's.

**Level matching survived the move, on both backends.** Previously
`keycodes_from_xmodmap` mapped *every* column of an `xmodmap` line to that
keycode, so `"S-colon"` and `"S-semicolon"` both worked; the keysym-keyed x11rb
backend preserves that by searching all levels of `GetKeyboardMapping` for the
named keysym. That is a deliberate divergence from xmonad, which is level 0 only on
both sides — `mkGrabs` builds its keysym map from `keycodeToKeysym dpy code 0`
and `handle` resolves a press the same way (`XMonad/Operations.hs`,
`XMonad/Main.hs`), so `xK_colon` grabs nothing on a US layout and EZConfig has
you write `M-S-;` instead. Penrose's existing behaviour is the more permissive
one and there is no reason to take it away. River is at least as permissive:
`Seat.matchXkbBinding` first matches without xkbcommon translation (so `M-S-1`
matches the keysym `1`) and then with it (so `M-S-exclam` and numlocked `KP_1`
match too), taking the first hit.

### Key sequences

Sequence bindings (`"M-m M-l"`, xmonad's `EZConfig` style) are implemented in
core and offered upstream as
[penrose#344](https://github.com/sminez/penrose/pull/344). The parts that are
river's problem:

**Every key of a sequence needs a binding object, not just the leaders.** Core
grabs `KeyBindings::leading_keys()` and expects the rest of a sequence to arrive
through the capture. On X11 that works because `grab_keyboard` reports the
keycode of whatever was pressed. River will not: `ensure_next_key_eaten` eats the
next non-modifier press, and if it *triggers a binding* the ordinary
`pressed` event is sent, but if it does not, the event is `ate_unbound_key`,
which carries no keysym. So the river backend has to enable the continuation keys
for the duration of the capture — and it can, because the signature is now
`capture_next_key(&mut self, continuations: &[Self::KeyBindingKey])`. That was
the one prerequisite outside the river work and it is **done**; x11rb takes the
same set for level matching, which closed the "a continuation must be named by
its first level" wart in the bargain.

**`ate_unbound_key` needs somewhere to go.** It means "a key was eaten and it
was not one of yours", which is exactly the abort signal, and core has no way to
say it: `dispatch_key` takes a `C::KeyBindingKey` and there is no value that
means "not a key". The backend needs an entry point that clears
`State::pending_keys` and drops the capture — one function beside `dispatch_key`,
with no trait change.

**Modifier filtering is river's job, not ours.** `ensure_next_key_eaten` is
specified over the next *non-modifier* press, so the river backend needs no
counterpart to x11rb's `modifier_keycodes` set (which exists precisely because
`grab_keyboard` reports `Super` presses too, and refreshing it on
`MappingNotify` is why `grab` re-reads the modifier map).

**Arming is atomic with the press, if the conn cooperates.**
`river_xkb_binding_v1.pressed` is followed by a `manage_start`, and "the
compositor should wait for the manage sequence to complete before processing
further input events" — so the enable/disable that the handler buffered lands
before river looks at the next key, with no window in which the outer bindings
are still live. That is better than X11 can do, and better than
`~/env/xmonad-river` achieves (its DESIGN.md records a ~100ms window in which the
wrong binding runs, a consequence of its worker thread rather than of the
protocol). The guarantee is only real if the conn does not finish the manage
sequence before penrose's handler has run — see §2.

**The capture is one-shot at both ends.** Core relies on the backend releasing
the capture when it delivers a key, so a caller that never cancels cannot wedge
the keyboard; `ensure_next_key_eaten` is one-shot by construction, which is the
same shape. `cancel_ensure_next_key_eaten` is itself manage state, and river
documents the race — `ate_unbound_key` may already be in flight — so the cancel
path has to tolerate arriving late.

**Clicks do not cancel a sequence**, on either backend. xmonad grabs the pointer
so a click cancels and is swallowed; river has no pointer counterpart to
`ensure_next_key_eaten`, and `river_seat_v1.window_interaction` is a
notification that cannot swallow the click. X11 would swallow it and river would
not, which is two behaviours sharing a name, so neither does. The cost is that a
click leaves a half-typed sequence armed and the next key press ends it.

**A window manager cannot read keys, and that is a protocol fact.** A binding
reports that it fired, never what was pressed; `river_seat_v1` has no key
events, which is right for a window manager and useless for a text field. So
anything prompt-shaped is a separate program by necessity as well as by scope —
`fuzzel`, via the existing `DMenu` util, once that util has a `fuzzel`
`DMenuKind` and a `build_menu` that does not block (§2, §10 step 5). X11 §9
already picked a menu program with a Wayland story; this makes that a
requirement rather than prudence.

## 6. Screens, borders, workspaces

**Screens.** `river_output_v1` gives `position` and `dimensions`;
`river_layer_shell_v1.get_output` gives `non_exclusive_area` per output, i.e.
the area left after bars claim their exclusive zones. `screen_details()` returns
the non-exclusive areas, sorted in the conn. Two things fall out: X11 §5's
`PhysConn` newtype is unnecessary — right-to-left physical ordering is a
constructor option on `RiverConn` — and `ReserveTop` becomes dead, because the
compositor already told us where the bar is.

The other half of that global's job is covered under Scope: binding it is what
stops river closing every layer surface on sight, whether or not the work area is
wanted. `set_default` — which output an unplaced layer surface lands on — is
manage state, so it lives in the plan (§2).

**Borders.** `river_window_v1.set_borders(edges, width, r, g, b, a)` — the
compositor draws them, in the render half of the plan.

**Fitting a border into a client's allocation is the backend's job**, because the
two backends disagree about what fitting means:

```rust
fn position_client(&mut self, id: WinId, r: Rect, border: u32) -> Result<()>;
```

`r` is the space the layout allocated, not the client's geometry. An X11 border is
drawn *outside* the window's origin, so x11rb shrinks the client by `2 * bw` and
the two together fill the allocation — the same correction orilla applies by hand
(`../orilla/crates/orilla/src/lib.rs` subtracts `2 * bw` from the proposed
dimensions and adds `bw` to the node position). River needs neither: it positions
the *content* and draws borders over the content's own edges, so the client fills
its allocation and the border eats the outermost pixels of it. That reproduces the
X11 picture rather than departing from it — neighbouring windows touch, so the
line between two of them is one border from each, and a window against the screen
edge shows one border's width there.

What stays in core is the policy rather than the arithmetic, since it needs the
screens: a client filling its whole screen is passed a width of 0, because there
is nothing on the other side of it for a border to separate it from. So the width
is per window, which is also why `RenderPlan` carries one per window rather than a
single global.

`tests/headless-river.sh` asserts the result the layout-independent way: whatever
split the layout chose, the widths of the windows sharing a screen add up to the
width of that screen.

Two details the protocol adds. The channels are 32-bit and premultiplied
(`0x00000000`–`0xffffffff` read as a percentage), so `Color`'s bytes scale up
rather than passing through. And river draws borders *above* the window content
rather than around it, so where X11 adds a ring outside the content, river eats
`border_width` of the content's edge — same geometry, slightly different picture,
and there is no way to ask for the X11 one.

**Workspaces.** River has no workspace concept. Hidden workspaces are
`river_window_v1.hide` / `.show` in the render sequence — the same one-line
answer in both existing implementations. `StackSet` is untouched.

## 7. Restart

X11 §2 restarts by exiting and letting a supervisor script relaunch, relying on
**the X server as the state store**: `manage_existing_clients` reads
`_NET_WM_DESKTOP` and `_NET_ACTIVE_WINDOW` back off each window. River has no
property store, so that route loses tags and focus outright — strictly worse
than the X11 build.

Use river's hot-swap instead: `M-q` rebuilds, writes a state file keyed on
`river_window_v1.identifier`, sends `river_window_manager_v1.stop`, and execs
itself on `finished`. River keeps every client alive across the swap. The
identifier is stable across a restart because it derives from the window's
`ext_foreign_toplevel_handle_v1`, which belongs to the window rather than to our
connection, and it arrives before the first `manage_start` — both are properties
of *river*, and neither is in the XML, which promises only uniqueness and
non-reuse. `river/Window.zig` sends `handle.identifier` from the window's
foreign-toplevel handle at the point the `river_window_v1` is created, which is
the source of the first; xmonad-river verified both against a headless
compositor. They are also why §3 insists on a protocol version that has the
event.

What does not survive is what does not survive under X11 either (layout state,
stacking order, extension state), for the same reason: `Box<dyn Layout>` is not
serializable.

`xmonad --restart`-style external triggering, if wanted, is a Unix socket at
`$XDG_RUNTIME_DIR/penrose-$WAYLAND_DISPLAY.sock` — named for the display so two
rivers get one window manager each. A socket rather than a pid file and a
signal, because it can carry a refusal back to the terminal that asked.

## 8. Status bar

`penrose_ui` draws in-process with X11 and Xft and does not port. Per Scope it
gets no Wayland equivalent either: publish a JSON snapshot
(tags, per-tag windows with `app_id`/`title`/focus, layout name) over a Unix
socket from a refresh hook, deduplicated by hash, and let an existing Wayland
bar render it. `../orilla/crates/orilla/src/ipc.rs` is 132 lines over
`serde_json` and `std::os::unix::net`, and this belongs in the config crate
(`~/env/penrose/src/`) rather than in penrose, so it adds no dependency to
either backend. waybar's custom module and yambar both consume exactly this.

This is smaller than the thing it replaces, works under X11 too, and is the one
place where river's constraints make the design better rather than harder.

## 9. What is absent under river

Stated explicitly, because the failure mode otherwise is discovering it at
runtime:

- **EWMH, atoms and window properties.** The whole `hooks/ewmh.rs` extension,
  and with it X11 §7's activation-suppression problem — river has no
  `_NET_ACTIVE_WINDOW` message for Chrome to send.
- **Property-based queries.** `ClassName` → `app_id`, `Title` → `title`;
  `AppName` collapses into `app_id`, since river has no separate instance name.
  `StringProperty` has no meaning. X11 §6's class-based placement survives
  intact — `alacritty --class syslog` sets `app_id` under Wayland too.
- **`send_client_message`.** X11 §10's answer for reaching the event loop from a
  thread. Under river it is the control socket from §7, or a `calloop` source.
- **`client_pid`** is `river_window_v1.unreliable_pid` — named for what it is.
  X11 §6's `isAutomatedBrowser` query degrades accordingly.
- **Urgency**, which penrose does not have anyway.

Manage hooks are *not* on this list, and it is worth knowing why: a window is
not displayed until the window manager has proposed dimensions in a manage
sequence and a render sequence has finished. So a manage hook runs before the
window is ever shown — the ordering guarantee xmonad has under X11 and that
sway's IPC cannot give.

## 10. Shape, order, open questions

```
vendor/penrose/src/river/
  mod.rs           -- RiverConn: the Conn impl, on penrose's thread (§2)
  wayland.rs       -- the loop: owns the connection, answers sequences (§2)
  shared.rs        -- the published plan and the compositor view between them (§2)
  plan.rs          -- ManagePlan, RenderPlan, Op, transmit + liveness filtering (§2)
  protocol.rs      -- wayland-scanner expansion (§3)
  protocol/*.xml   -- copies, with a SOURCE file naming the river commit (§3)
  event.rs         -- RiverEvent (§4)
  bindings.rs      -- binding objects per KeySym, per seat; loop-owned (§5)

vendor/penrose/tests/
  headless-river.sh     -- a layout reaches the compositor, two outputs (§3)
  headless-bindings.sh  -- bindings and key sequences, via a virtual keyboard (§5)
  headless-menu.sh      -- a handler blocks on fuzzel and survives (§2)
  headless-restart.sh   -- clients and their tags survive a hot swap (§7)
```

Both prerequisites that sat outside the river work are done. Key sequences in
core are up as [penrose#344](https://github.com/sminez/penrose/pull/344), and the
continuation set they hand the backend landed alongside; what remains of either
is river-side and lives in §5.

0. ~~Keysym-keyed bindings (§5)~~ — **done.** `KeyBindingKey` is keysym + mask on
   every backend, x11rb resolves to keycodes at grab time, `xmodmap` is gone.
   First because it was an X11 bug fix that stood alone, and because settling
   `KeyBindingKey` with one backend in the tree was cheaper than with two.
1. ~~Feature-flag hygiene (§1)~~ — **done.** `--no-default-features --features
   river` builds a crate with no X11 in it.
2. ~~Protocol copies and expansion (§3), and a headless river test harness~~ —
   **done**, as `tests/headless-river.sh`, modelled on
   `~/env/xmonad-river/tests/headless-river.sh`. The assertion that matters is
   river's own log line reporting a tracked `configure` — the only signal that
   proves a layout reached the compositor.
3. ~~`RiverConn` with a plan~~ — **done.** Two windows tile on the first run.
4. ~~Bindings (§5), then workspaces and hide/show (§6)~~ — **done**, with
   `tests/headless-bindings.sh` pressing keys through a virtual keyboard, since
   a headless seat has none. It found the bug it was written for: only the
   leaders had binding objects, so every sequence continuation died as
   `ate_unbound_key`.
5. ~~Let handlers block~~ — **done**, and not the way this file first said. The
   no-blocking contract was replaced by the two-thread split (§2), so a config's
   existing blocking menus, prompts and rebuilds work unaltered.
   `tests/headless-menu.sh` blocks a handler on a real `fuzzel` and gets the
   choice back.
6. ~~Multi-output and the non-exclusive area (§6)~~ — **done**; the harness runs
   river with two headless outputs.
7. ~~Restart (§7)~~ — **done**, with `tests/headless-restart.sh` asserting the
   two things about river the approach rests on: a hot swap keeps clients alive,
   and the window identifier survives it. The socket status feed (§8) belongs in
   the config crate and is not part of this fork.

**Future work.**

- **An automatic check that the vendored XML is up to date** (§3). Fetch the
  protocol files from river's tip, diff against `src/river/protocol/`, and fail
  loudly with the diff — as a test, so it runs where everything else runs, and
  skipping cleanly when the network is unavailable rather than failing the
  offline build. Bumping is then a copy plus whatever the diff makes necessary,
  and the `SOURCE` commit moves with it. Without this the copies rot silently and
  the first symptom is a protocol error against a river that has moved on.
- **A `fuzzel` `DMenuKind`** (§5), alongside the existing `Suckless` and `Rust`
  variants, which are X11 dmenus with dmenu's flags. Part of step 5 rather than
  separate from it: a non-blocking `DMenu` a river config cannot invoke is only
  half the job.

**Open questions.**

- **How long should the waits be?** `PLAN_WAIT` is 20ms and `KEY_WAIT` 200ms,
  both picked rather than measured. Too short and a key sequence arms a round
  trip late; too long and a blocked handler stalls the compositor's input for
  that long before the loop gives up on it. xmonad-river records the same
  question unanswered.
- **There is still no wakeup path into the loop**, which §7's external restart
  trigger and §8's status feed both want: a thread with something to say has no
  way to make the worker notice. It is much cheaper now than it was — the loop
  already exists and events already arrive on a channel — so it is a matter of
  giving the worker something to select on rather than a redesign.
- **A dead worker under a live loop looks alive.** The loop would keep answering
  sequences from the last plan, so the session would look fine and respond to
  nothing. A panicking handler takes the process down today, which is the right
  outcome by accident rather than by design.

Five smaller things the implementation decided that this file did not, each of
which is defensible and none of which was thought about for long:

- **`set_default` is not configurable.** The `ManagePlan` sketched above has a
  `layer_default` field; the implementation hardcodes "the first output with a
  usable area". That is which screen a layer surface lands on when it does not
  ask for one — a bar's menu, a notification — so on a multi-monitor setup
  somebody will eventually want to say which.
- **`client_is_fullscreen` reports what we asked for, not what happened.** The
  protocol has no query for it, so the plan is the only answer available, but it
  is optimistic: a window we asked river to fullscreen reads as fullscreen even
  if river could not oblige.
- **Focus goes to every seat.** Penrose has one focused window, and the conn
  currently tells each seat about it, so a genuinely multi-seat river would have
  every seat following the same window. Nothing in penrose has a place to say
  otherwise, which is the real gap.
- **`requires_pointer_warp` is false for `Interaction` as well as
  `PointerFocus`.** §4 says only `PointerFocus`, but a click that yanks the
  pointer to the middle of the window it just clicked is obviously wrong, so
  `window_interaction` is treated the same way. Worth stating rather than
  leaving as a silent divergence from the X11 `Enter` rule it is modelled on.
- **`penrose_ui` is still a workspace member.** §1 says it is "simply not built
  under `river`", which is true of `cargo build -p penrose --features river` and
  false of `cargo build --workspace`: the workspace build includes it, and its
  dependency on penrose with default features unifies `x11rb` back on. Harmless,
  since the two features are additive by design, but it means a river-only build
  is a package build rather than a workspace one.
- **Protocol churn.** `river-window-management-v1` is unstable and untagged. The
  pinned-commit generator (§3) is the mitigation; there is no version at which
  this stops needing attention.
- **Floating windows.** `river_seat_v1.op_start_pointer` is river's interactive
  move/resize, and it is not the same shape as X11 §8's `MouseDragHandler`
  (which computes positions itself from motion events). Unclear whether
  penrose's floating layer maps onto it or should ignore it and drive positions
  directly.
- **XWayland.** River supports it and `river_window_v1` covers XWayland windows,
  so nothing special is needed — but nothing has verified that.
