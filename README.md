Penrose - a tiling window manager in the style of dwm
=====================================================

Not ready yet for general use but aiming for a configure in source and recompile
model similar to dwm / Xmonad / Qtile. The code should be well documented and
relatively easy to understand (if not, please let me know!): I'm learning the
XCB API as I go so there are likely multiple places where things are not being
done in the smartest way.

### Non-goals
- A config file
  - Parsing a config file and dynamically switching behaviour on the contents
  adds a huge amount of complexity to the code. I'd much rather keep the code
  simple and well documented so that modifying it is easy and then just
  recompile. This is also more in the spirit of dwm (some top level config for
  quick changes but really you should dig into the source) than Xmonad/Qtile (a
  library that you use to write your on WM) but I suspect the latter aproach
  should be possible.

- IPC / relying on external programs
  - I love acme from plan9 and how easy it is to drive it's state from external
  programs (check out my [acme-corp](https://github.com/sminez/acme-corp) tools
  to see what I mean) but that comes at the expense of the internal logic
  becoming _massively_ more complicated. As few moving parts as possible is
  ideal. So, things that are easy to acomplish using the XCB api (key bindings,
  simple rendering of a bar etc) are in, full on IPC via an exposed API is out.

- Programatic hooks
  - Rather than expose a set of hooks to be triggered, it is encouraged that you
  simply modify the WindowManager method directly. Want to trigger something
  every time you switch to a new workspace? Add a custom function call to the
  `switch_workspace` method. Done.


### Current TODO list
- [x] Focus
  - Need to track the focused client (on focused monitor only) both for adding a
  visual indicator but also so that the client can be manipulated by other
  actions from the `WindowManager`.

- [x] Handling client removal
  - Should(?) just be a case of firing off the correct xcb messages to trigger
  this and then exposing that as an action. (Relies on being able to track the
  currently focused client though!)

- [ ] Workspaces
  - I've started writing penrose with the idea that I would use a tag based
  system similar to dwm as that is what I am currently using. Now that I think
  about it some more, I much prefer the set up I used to have with Qtile where I
  added in the ability to "throw" workspaces from one monitor to another and
  modify layout settings per workspace.
  - With that in mind, there would be a single client list, with each workspace
  tracking a list of known clients. Monitors would then have an active workspace
  and user bindings would trigger modifications to those mappings rather than
  modifying the client structs themselves.

- [ ] dwm style bar and systray
  - This one is probably a decent amount of work...I still prefer it to having
  to install and configure something like lemonbar/polybar though and exposing
  the API via xsetroot is a nice touch that I'd like to keep.
  - While the Qtile bar I used to have was my favourite by far, I doubt I'm
  going to be able to reproduce that any time soon starting from scratch!

- [ ] Scratchpads
  - My dwm set up currently has a single scratchpad (a terminal) but I've
  previously has a set up where different bindings triggered scratchpads for
  different programs. Not decided yet on whether I prefer that (hard coded
  scratch pads) or the ability to tag windows as being on a scratch pad and then
  cycling through them a-la i3.

- [ ] Mouse bindings
  - I'm currently grabbing `MOD-{1, 3}` on start up with the intension of using
  that for mouse based move and resize respectively. The xcb events provide
  cursor position info so it should be a case of generating a target region from
  that and applying the resize.
  - Probably should factor out the resize logic so this can be reused then...
  - getting draw-term back (possibly as a build in feature) would be pretty cool
