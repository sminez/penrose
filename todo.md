- Keep the `xmodmap` path as a fallback for a name the generated table has no
  keysym for, and complain loudly when it is used.

  `XKeySym` covers about 680 of the ~2300 names in `keysymdef.h` and
  `XF86keysym.h`, which was plenty when it only backed the utf8 conversion and
  is now the set of names that parse. `xmodmap -pke` reports whatever is in the
  current keymap, so it can resolve names the table lacks -- `XF86Sleep`, the
  `XF86Launch*` keys, any `dead_*` key, `Multi_key` -- and a config that used to
  work should not stop working because the generator has not been re-run.

  A fallback rather than a return: it should log at warn naming the binding and
  the name, and say that the table is what should be regenerated. Otherwise the
  subprocess comes back for good and the parse stops being pure, which is most
  of the reason it moved.

- Starting while another window manager is running is not refused.

  `set_client_attributes` (`src/x11rb/mod.rs:735`) asks for
  `SUBSTRUCTURE_REDIRECT` on the root with an unchecked
  `change_window_attributes`. Only one client may hold that mask, so a second
  window manager gets `BadAccess` — but nothing waits for it, so startup
  proceeds: keys are grabbed, existing clients are managed, and the error only
  turns up later as `Unhandled error pulling next event` from the run loop,
  once the damage is done. Should be the checked variant, failing `run` with
  "another window manager is already running" the way xmonad, dwm and i3 do.

- `WmHints` parses the urgency flag and then hides it: `flags` is `pub(crate)`
  with no accessor, so a config cannot ask whether a window set it.

  Penrose has no urgency concept of its own — nothing acts on the flag — which
  makes this the one thing standing between a config and implementing it.
  `pub fn is_urgent(&self) -> bool`, or exposing `flags`, is the whole fix.

- Look at X11 error handling in general, of which the above is one instance.

  Every request in the x11rb backend is fire-and-forget (30 call sites, none
  `.check()`ed), so protocol errors surface asynchronously through
  `next_event`, detached from the request that caused them. `convert_event`
  turns them into `Err`, `handle_error` recognises exactly one — `BadWindow`,
  as `UnknownClient` — and everything else is logged and dropped.

  That is the right default for most requests, since a window can die between
  the check and the request, but it is wrong for the ones whose failure is a
  startup condition rather than a race. Worth deciding per request which is
  which, and giving the errors that matter a path that reaches the caller.
  `handle_error` catching `Access` on the root would be a cheap first step even
  without checked requests.

  `grab` is done, and is the case that argues for the rest. Another client
  holding a key -- ibus takes `Super+space` for switching input method, and is
  started before the window manager by `im-launch` on Debian -- makes `GrabKey`
  answer `BadAccess`. Unchecked, that was dropped and the binding silently never
  fired, with nothing anywhere to say why; checked, it is a warning naming the
  keysym. Every other request whose failure means "this will never work" rather
  than "the window went away" deserves the same.

- Nothing has run against a real X server yet: the keysym work is unit tested
  against a hand written keymap only. Xephyr should confirm a plain binding
  fires, `S-semicolon` and `S-colon` both fire, a sequence completes and a wrong
  key aborts it, and the media keys work.

- Upstream: river un-hides every window when the window manager disconnects,
  which is worth fixing in river rather than only working around here.

  `Window.zig`'s `handleDestroy` (river `67379a2`) resets `rendering_requested`
  with `hidden = false`, and `WindowManager.handleDestroy` calls it for every
  window. So between the outgoing window manager exiting and the replacement's
  first render plan, the entire session is on screen at once, stacked at
  whatever positions it last had. A hot swap is meant to be the invisible way to
  replace a window manager, and this is the one part of it that is not.

  It is not only cosmetic. `Cursor.updateHovered` runs off the scene, from
  `updateState` after every change, so the seat's hovered window is recomputed
  against that pile; `Seat.manageStart` then sends it to the replacement as
  `pointer_enter` in its initial state. The new window manager is told the
  pointer is over a window that is only under it because everything was
  un-hidden -- which, with focus-follows-mouse, moved this config to the
  workspace of whatever was on top at every `M-q`. Worked around in
  `src/river/mod.rs` by ignoring an enter for a window that is not on a visible
  tag, but the compositor should not be reporting it.

  The fix is presumably to keep the last rendering state until a replacement's
  first `render_finish` rather than reverting it on disconnect. The wrinkle is
  the case with no replacement coming: keeping windows hidden then leaves a
  session with nothing on screen and no way to get anything back, which is
  probably why it reverts. A grace period, or reverting only once the window
  manager global has gone unbound for some time, would keep both.

  Worth asking on the river issue tracker before writing it: the maintainer may
  want the initial `pointer_enter` deferred until the first render instead,
  which fixes the focus half without touching visibility.
