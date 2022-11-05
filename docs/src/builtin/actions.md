# Actions

When it comes to extending the behaviour of you window manager, the first and most
obvious thing to look at is running some custom code in response to a key binding
being pressed. In penrose, this is refered to as an `action`.

Actions can be anything from focusing a new window, to changing the layout algorithm
being used, to opening a terminal or running fully custom logic to find and display
amusing pictures of squirrels.

The choice is yours.

To help with some of the boilerplate and common cases, there are a couple of helper
functions that will generate a `KeyEventHandler` for you in a relatively simple
way. There are also a couple of built in actions for working with floating windows
and exiting penrose to get you started.


## Writing actions using helpers

There are five helper functions for writing common actions:

  - `key_handler`: this one is the most general. It wraps a function that takes a
    mutable reference to the current window manager state and a reference to the
    `XConn` used by your window manager and runs whatever custom code you care to
    write.
  - `modify_with`: for calling `pure` state methods this helper handles the diff
    and refresh cycle for you. Simply update the `StackSet` with whatever changes
    you want to make and a refresh will be triggered for you to reflect you changes
    to the X server.
  - `send_layout_message`: this does pretty much what you'd expect. It calls the
    given function to construct a `Message` and sends it to the active layout.
    (Useful for updating your layout behaviour on the fly).
  - `broadcast_layout_message`: does the same thing as `send_layout_message` only
    in this case the message is copied and sent to _all_ layouts available to the
    current workspace rather than just the active one.
  - `spawn`: as the name implies, this spawns a given program as a subprocess.
    You probably want at least one key binding for spawning either a terminal or
    a program launcher such as `dmenu` or `rofi`. For the programs you use the
    most, this lets you get to them with a single key press!
