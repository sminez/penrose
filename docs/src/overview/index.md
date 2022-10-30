<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Overview of Concepts

Penrose is a [dynamic tiling window manager][0] for [Xorg][1] in the spirit of [Xmonad][2]. Most of the
concepts and APIs you'll find for penrose are nothing new, but if you plan on digging into writing your
own window manager then it's worthwhile taking a bit of time to learn what all the moving parts are.

At its core, the main operation of penrose is an event loop that reacts to events received from the X
server. In simplified rust code, it looks something like this:
```rust
loop {
    let event = get_next_xevent();
    match event {
        // for each event type run the appropriate handler
    }
}
```

There's obviously more to it than that, but this is a pretty good starting point for how to think about
your window manager. Penrose provides a number of different ways to modify how the default handling
of events behaves and for running custom code in response to key presses. The pages in this section of
the book each cover (at a relatively high level) what the moving parts that make this work all look like.

First up: pure code vs X code.


  [0]: https://wiki.archlinux.org/title/Window_manager
  [1]: https://wiki.archlinux.org/title/Xorg
  [2]: https://xmonad.org/
