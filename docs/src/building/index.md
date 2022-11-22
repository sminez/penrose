<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Building on top of penrose

Out of the box, the examples provided in the penrose GitHub repository show you how to put
together a fairly minimal window manager. By design, penrose does not attempt to implement
every piece of functionality you might like from your favourite window manager, instead it
provides a set of rich, composable APIs for extending the behaviour and adding your own
custom logic.

The simplest place to start is with running custom code in response to key bindings, whether
that's to modify how your windows are arranged on the screen, to launch a new program or
run completely custom logic. From there you can dig into things like custom layout algorithms
and extending the core window manager behaviour with hooks.

> If you've ever experimented with Xmonad or Qtile before then the set up should feel
> somewhat familiar to you.
