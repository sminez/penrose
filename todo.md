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

  Every request in the x11rb backend is fire-and-forget (31 call sites, none
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

- Nothing has run against a real X server yet: the keysym work is unit tested
  against a hand written keymap only. Xephyr should confirm a plain binding
  fires, `S-semicolon` and `S-colon` both fire, a sequence completes and a wrong
  key aborts it, and the media keys work.
