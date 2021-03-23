<image width="60px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Getting Started With Penrose
<br>

So, you've heard about Penrose and you'd like to take it for a spin. Maybe
you've watched one of the [YouTube][0] videos, or found it on [crates.io][1]
while searching for window managers. The following is a quick guide for how to
get your system set up for building your window manager binary and running it.

<br>

### Step 0. Programming basics

Before reading any further, please understand one very important thing about
Penrose:

<b>Penrose is a window manager library, not a program.</b>

This means that in order to use Penrose as your window manager, you will need to
write some code. A quick, minimal set up (what we will build out now) is
possible with relatively little programming experience, but you _will_ be
interacting with the Rust compiler and build tools (such as [cargo][2]).

It is _strongly_ recommended that you read through the [rust book][3] and try
some of the examples there before diving too deep into writing your window
manager with Penrose. It is worthwhile taking a look at the [learn][4] section
of the Rust website if you have never used Rust before. There is a bit of a
learning curve (as with all programming languages) but once you get started you
will likely find yourself quickly hooked!

<br>

### Step 1. Install Rust

If you have Rust on your system already, congrats! You can skip this section.

For everyone else, head on over to [rust-lang.org](https://www.rust-lang.org/)
and click on the big "Get Started button" which will advise you to curl a
setup script straight into **sh**. If you'd prefer to see what you are about to
run, the following should do the trick:

```bash
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh

# Open and peruse in your editor of choice
$ vim rustup.sh

# Then, to carry out the actual install
$ chmod +x rustup.sh
$ ./rustup.sh
```

Now simply sit back and wait for while Rust is installed on your system.

<br>

### Step 2. Create a new crate

"Crates" are Rust's term for what you might be more used to calling a package or
project. Either way, for your window manager you are going to want to make a new
_binary_ crate like so:

```bash
$ cargo new --bin my_penrose_config
$ cd my_penrose_config
$ exa -T
.
├── Cargo.toml
└── src
   └── main.rs
```

<br>

### Step 3. Add dependencies

You should now have a new git repo with the content shown above. At the moment,
your **main.rs** is just a simple Hello World program (we'll be fixing that
soon) but first, open up **Cargo.toml** and add Penrose as a dependency. While
you're at it, it's worthwhile adding a logging handler as well so that you can
see what Penrose is up to, should you encounter any issues (I prefer simplelog
but feel free to use whatever you wish):

_Cargo.toml_
```toml
[package]
name = "my_penrose_config"
version = "0.1.0"
authors = ["You <you@your-email-provider.com>"]
edition = "2018"

# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

[dependencies]
penrose = "0.2"
simplelog = "0.8"
```

<br>

### Step 4. Write your main.rs

The following snippet is a good example of a minimal **main.rs** that will give
you functionality similar to that of a minimal tiling window manager such as
**dwm**. Have a look at the [documentation][5] on **doc.rs** to learn more about
what each function and data structure is doing:

_src/main.rs_
```rust
#[macro_use]
extern crate penrose;

use penrose::{
    core::{
        bindings::KeyEventHandler,
        config::Config,
        helpers::index_selectors,
        manager::WindowManager,
    },
    logging_error_handler,
    xcb::new_xcb_backed_window_manager,
    Backward, Forward, Less, More, Selector
};

use simplelog::{LevelFilter, SimpleLogger};


// Replace these with your preferred terminal and program launcher
const TERMINAL: &str = "alacritty";
const LAUNCHER: &str = "dmenu_run";


fn main() -> penrose::Result<()> {
    // Initialise the logger (use LevelFilter::Debug to enable debug logging)
    if let Err(e) = SimpleLogger::init(LevelFilter::Info, simplelog::Config::default()) {
        panic!("unable to set log level: {}", e);
    };

    let config = Config::default();
    let key_bindings = gen_keybindings! {
        // Program launchers
        "M-semicolon" => run_external!(LAUNCHER);
        "M-Return" => run_external!(TERMINAL);

        // Exit Penrose (important to remember this one!)
        "M-A-C-Escape" => run_internal!(exit);

        // client management
        "M-j" => run_internal!(cycle_client, Forward);
        "M-k" => run_internal!(cycle_client, Backward);
        "M-S-j" => run_internal!(drag_client, Forward);
        "M-S-k" => run_internal!(drag_client, Backward);
        "M-S-f" => run_internal!(toggle_client_fullscreen, &Selector::Focused);
        "M-S-q" => run_internal!(kill_client);

        // workspace management
        "M-Tab" => run_internal!(toggle_workspace);
        "M-A-period" => run_internal!(cycle_workspace, Forward);
        "M-A-comma" => run_internal!(cycle_workspace, Backward);

        // Layout management
        "M-grave" => run_internal!(cycle_layout, Forward);
        "M-S-grave" => run_internal!(cycle_layout, Backward);
        "M-A-Up" => run_internal!(update_max_main, More);
        "M-A-Down" => run_internal!(update_max_main, Less);
        "M-A-Right" => run_internal!(update_main_ratio, More);
        "M-A-Left" => run_internal!(update_main_ratio, Less);

        refmap [ config.ws_range() ] in {
            "M-{}" => focus_workspace [ index_selectors(config.workspaces().len()) ];
            "M-S-{}" => client_to_workspace [ index_selectors(config.workspaces().len()) ];
        };
    };

    let mut wm = new_xcb_backed_window_manager(config, vec![], logging_error_handler())?;
    wm.grab_keys_and_run(key_bindings, map!{})
}
```

Here we are using the default **xcb** based back end for Penrose which will
require you to install the C xcb library on your system. Currently, this is the
only backend offered "out of the box", though it is possible to write your own
should you wish.

<br>

### Step 5. Compile and run

Compiling your new window manager is pretty simple:

```bash
$ cargo build --release
```

You'll see a lot of output the first time round as the dependencies of Penrose
itself are compiled, but after that your re-compile should be pretty quick
following any changes that you make. Once the binary is compiled you should see
it as a new executable in **target/release**.

My preferred way of running Penrose (and my system) is to login via TTY and
place the following in my **~/.xinitrc**:

_.xinitrc_
```bash
/home/innes/.config/penrose/target/release/penrose &> ~/.penrose.log
```

Then, I can simply type **startx** after logging in and Penrose will start up,
with my current session log available in my home directory as **.penrose.log**.

If logging in and starting your graphical session from a TTY is too hipster for
you, you might want to look at installing and running a [display manager][6]
which will require you writing a Desktop Entry for Penrose. Details of how to do
this (for any program) are available online.

<br>

### Step 6. Profit

And that's it!

You are now the proud owner of a shiny new window manager. From this point on
you can start tinkering to your heart's content and setting things up exactly
how you want. Speaking from personal experience, I would advise that you commit
your changes to your window manager _regularly_ and that you make sure you know
how to revert to your last good state in case you manage to introduce any
particularly nasty bugs into your setup. If that happens, simply rebuild your
previous good state and get to work on fixing your bug.

Happy coding!


  [0]: https://www.youtube.com/channel/UC04N-5DxEWH4ioK0bvZmF_Q
  [1]: https://crates.io/crates/penrose
  [2]: https://doc.rust-lang.org/book/ch01-03-hello-cargo.html
  [3]: https://doc.rust-lang.org/book/title-page.html
  [4]: https://www.rust-lang.org/learn
  [5]: https://docs.rs/penrose
  [6]: https://wiki.archlinux.org/index.php/Display_manager
