<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Getting started

So you'd like to manage your windows, maybe even tile them?

Well aren't you in luck!

The following is a quick guide for how to get your system set up for building a penrose based
window manager and getting it running. By the end of this guide you will have a _very_ minimal
window manager that you can use as a starting point.

If you've ever tried out [xmonad][0] before then the overall design and feel of how penrose works
should feel (somewhat) familiar. The key thing is this: penrose is a library for writing a window
manager. It's not a pre-built window manager that you then configure via a config file. In practical
terms what that means is that it's time to get our hands dirty with writing some code!


## Step 0: Getting set up with Rust

If you have Rust on your system already, congrats! You can skip this section.

For everyone else, head on over to [rust-lang.org][1] and click on the big "Get Started button"
which will advise you to curl a setup script straight into **sh**. If you'd prefer to see what
you are about to run, the following should do the trick:

```bash
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rustup.sh

# Open and peruse in your editor of choice
$ $EDITOR rustup.sh

# Then, to carry out the actual install
$ chmod +x rustup.sh
$ ./rustup.sh
```

Now simply sit back and wait for while Rust is installed on your system.


## Initialising your window manager crate

"Crates" are Rust's term for what you might be more used to calling a package or project.
Either way, for your window manager you are going to want to make a new _binary_ crate like so:

```bash
$ cargo new --bin my_penrose_config
$ cd my_penrose_config
$ exa -T  # or just plain old 'ls' if you prefer
.
├── Cargo.toml
└── src
   └── main.rs
```

If you open up `main.rs` you should see a simple hello world program:
```rust
fn main() {
    println!("Hello, world!");
}
```

We can run this using `cargo run` to check everything is good to go:
```bash
$ cargo run
   Compiling example v0.1.0 (/home/roger/my_penrose_config)
    Finished dev [unoptimized + debuginfo] target(s) in 0.43s
     Running `target/debug/example`
Hello, world!
```

Nice! Time to write a window manager.


## Writing your main.rs

You should now have a new git repo with the content shown above. At the moment,
your **main.rs** is just a simple Hello World program (we'll be fixing that soon)
but first we need to add some dependencies. Most importantly, we need to add
penrose itself but we're also going to add another crate as well to make our lives
a little easier: [tracing-subscriber][2] can be used to collect and process the logs
that penrose generates as it runs. It's by no means _required_ to collect the logs
but it definitely helps track down issues with your window manager if you can see
what's going on inside as it runs!

Thankfully, adding new dependencies is also something cargo can handle for us! It
even handles enabling optional features for us (which is handy, because we need to
do just that with `tracing-subscriber`):

```bash
$ cargo add tracing-subscriber --features env-filter
$ cargo add penrose
```

With that done, we're going to copy the [minimal example][3] from the penrose repository in
GitHub as our window manager. Either copy and paste the contents of the example into your
`main.rs` or (my prefered choice) use wget to pull it directly from GitHub:
```bash
$ cd src
$ rm main.rs
$ wget https://raw.githubusercontent.com/sminez/penrose/develop/examples/minimal/main.rs
```

For reference, your `main.rs` should now look like this:
```rust
{{ #include ../../examples/minimal/main.rs }}
```

### Checking we're good to go

Hopefully you've spotted that the end of the example includes a test. Now, it's entirely up
to you whether or not you keep (and run) the test but it's highly recommended that you do.
It's actually recommended that you write _more_ tests for your window manager as you extend
the features you want and write your own custom code!

Penrose itself has a pretty comprehensive test suite of the main logic and provides a variety
of ways for you to check and confirm that things are behaving in the way that you expect.
To run our test (and any others that you have added yourself) we simply need to run `cargo test`:

> **NOTE**: This test (and the example itself) require you to have the [xmodmap][4] utility
> installed on your system in order to parse our keybindings.
>
> Make sure you have it installed before going further!


```bash
$ cargo test
    Finished test [unoptimized + debuginfo] target(s) in 0.03s
     Running unittests src/main.rs (target/debug/deps/example-1562870d47d380ed)

running 1 test
test tests::bindings_parse_correctly_with_xmodmap ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

You'll see a lot more output the first time you run the test as things are compiled, but so long
as you see the test passing we should be good to take things for a spin!



## Making use of our new window manager

So far we've been building things in `debug` mode, which is exactly what we want when we're testing
things out and running any test suites we have. For actually making use of our new window manager
though, we want to switch to `release` mode:
```bash
$ cargo build --release
```

You'll see a lot of output the first time round as the dependencies of Penrose itself are compiled,
but after that your re-compile should be pretty quick following any changes that you make. Once the
binary is compiled you should see it as a new executable in the `target/release` directory.

The simplest way of running your new window manager is to login via a TTY and place the following
in your `~/.xinitrc`:
```bash
exec /home/roger/my_penrose_config/target/release/my_penrose_config &> ~/.penrose.log
```

Then, you can simply type `startx` after logging in and your window manager will start up, with the
log output available in my your home directory as `.penrose.log`!

If logging in and starting your graphical session from a TTY is "too hipster" for you (which, lets be
honest, it is), you might want to look at installing and running a [display manager][6] and using
something like [xinit-session][6] to make this a little nicer. Alternatively, dropping something like
the following into `/usr/share/xsessions` should also do the trick. The [arch wiki][7] is a fantastic
place to read up on how to do these sorts of things whether you use Arch Linux or not.

```desktop
[Desktop Entry]
Version=1.0
Name=Penrose Session
Comment=Use this session to run penrose as your desktop environment
Exec=sh -c "/home/roger/my_penrose_config/target/release/my_penrose_config &> ~/.penrose.log"
Icon=
Type=Application
DesktopNames=Penrose
```

## Profit

And that's it!

You are now the proud owner of a shiny new window manager. From this point on you can start tinkering
to your heart's content and setting things up exactly how you want. Speaking from personal experience,
I would advise that you commit your changes to your window manager _regularly_ and that you make sure
you know how to revert to your last good state in case you manage to introduce any particularly nasty
bugs into your setup. If that happens, simply rebuild your previous good state and get to work on
fixing your bug.

The [xephyr.sh][8] script in the GitHub repository can be used to run a given example in an embedded
X session if you want a little bit of safety while you sanity check changes that you are making.
Details of how it works are in a comment at the top of the script and `examples/local_test` is git
ignored for you to be able to have a place to try things out.

The rest of this book goes on to cover some of the inner workings of the main library, how to write
and work with extensions and generally have fun tinkering with your window manager.

Happy window managing!


  [0]: https://xmonad.org/
  [1]: https://www.rust-lang.org/
  [2]: https://crates.io/crates/tracing-subscriber
  [3]: https://github.com/sminez/penrose/blob/develop/examples/minimal/main.rs
  [4]: https://wiki.archlinux.org/title/Xmodmap
  [5]: https://wiki.archlinux.org/title/Display_manager
  [6]: https://wiki.archlinux.org/title/Display_manager#Run_~/.xinitrc_as_a_session
  [7]: https://wiki.archlinux.org/title/Display_manager#Session_configuration
  [8]: https://github.com/sminez/penrose/blob/develop/scripts/xephyr.sh
