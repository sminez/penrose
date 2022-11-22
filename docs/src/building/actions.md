<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Actions

To start with we're going to assume that when we talk about running an `Action` we're talking about
executing some custom code in response to a key binding bein pressed. With that in mind, lets take
a look at the [KeyEventHandler][0] trait found in `penrose::core::bindings`:

```rust
pub trait KeyEventHandler<X: XConn> {
    fn call(&mut self, state: &mut State<X>, x: &X) -> Result<()>;
}
```

There's not much to it: you are given mutable access to the window manager `State` and a reference to
the X connection. From there you can do pretty much whatever you like other than return data (we'll
take a look at how you can persist and manage your own state in a bit!)

To make things easier to work with (and to avoid having to implement this trait for every piece of
custom logic you want to run) there are several helper functions provided for wrapping free functions
of the right signature.

> **NOTE**: In any case where you do not need to manage any additional state, it is _strongly_
> recommended that you make use of these helpers to write your actions as simple functions rather
> than structs that implement the `KeyEventHandler` trait.


## Built-in helpers

In the [penrose::builtin::actions][1] module you will find a number of helper functions for writing
actions. The most general of these being `key_handler` which simply handles plumbing through the
required type information for Rust to generate the `KeyEventHandler` trait implementation for you.

### An example
As a real example of how this can be used, here is the power menu helper I have in my own set up
which makes use of the dmenu based helpers in [penrose::extensions::util::dmenu][2] to prompt the
user for a selection before executing the selected action:
```rust
use penrose::{
    builtin::actions::key_handler,
    core::bindings::KeyEventHandler,
    custom_error,
    extensions::util::dmenu::{DMenu, DMenuConfig, MenuMatch},
    util::spawn,
};
use std::process::exit;

pub fn power_menu<X: XConn>() -> KeyEventHandler<X> {
    key_handler(|state, _| {
        let options = vec!["lock", "logout", "restart-wm", "shutdown", "reboot"];
        let menu = DMenu::new(">>> ", options, DMenuConfig::default());
        let screen_index = state.client_set.current_screen().index();

        if let Ok(MenuMatch::Line(_, choice)) = menu.run(screen_index) {
            match choice.as_ref() {
                "lock" => spawn("xflock4"),
                "logout" => spawn("pkill -fi penrose"),
                "shutdown" => spawn("sudo shutdown -h now"),
                "reboot" => spawn("sudo reboot"),
                "restart-wm" => exit(0), // Wrapper script then handles restarting us
                _ => unimplemented!(),
            }
        } else {
            Ok(())
        }
    })
}
```

The window manager state is used to determine the current screen (where we want to open dmenu)
but other than that we're running completely arbitrary code in response to a keypress. The main
thing to keep in mind is that penrose is _single threaded_ so anything you do in an action must
complete in order for the event loop to continue running.

### StackSet manipulation

The most common set of actions you'll want to perform are modifications to the `StackSet` in
to reposition and select windows on the screen. There are [a large number of methods][3] available
for modifying the current state of your windows and the [modify_with][4] helper gives you an
easy way to call them directly. If you think back to the minimal example window manager we covered
in the "getting started" section, we saw this in use for most of the key bindings. Paraphrasing
a little, it looks like this:
```rust
use penrose::builtin::actions::modify_with;

// Select the next available layout algorithm
modify_with(|cs| cs.next_layout());

// Close the currently focused window
modify_with(|cs| cs.kill_focused());
```

  [0]: https://sminez.github.io/penrose/rustdoc/penrose/core/bindings/trait.KeyEventHandler.html
  [1]: https://sminez.github.io/penrose/rustdoc/penrose/builtin/actions/index.html
  [2]: https://sminez.github.io/penrose/rustdoc/penrose/extensions/util/dmenu/index.html
  [3]: https://sminez.github.io/penrose/rustdoc/penrose/pure/struct.StackSet.html
  [4]: https://sminez.github.io/penrose/rustdoc/penrose/builtin/actions/fn.modify_with.html
