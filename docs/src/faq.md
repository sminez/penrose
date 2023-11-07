<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Penrose FAQs

## How do I install Penrose?
You don't: Penrose is a library that you use to write your own window manager.
Take a look at the getting started guide for details of how to use Penrose
as a library.
<br><br>


## Where can I view the Penrose source code?

Penrose is developed openly on [GitHub][0] and published to [crates.io][1]
periodically as new features are added. The `develop` branch always has the
latest code and is what I use for running Penrose on my personal laptop. It
is not advised that you pin your use of Penrose to the GitHub `develop` branch
as a typically end user however: breaking changes (and weird and wonderful bugs)
are highly likely.

You have been warned!
<br><br>


## How does Penrose differ from other tiling window managers?

Penrose is a tiling window manager _library_ rather than a tiling window
manager. It provides core logic and traits (interfaces) for writing your own
tiling window manager, along with default implementations that can be used out
of the box. That said, you can't **install** Penrose: you need to write your own
Rust [crate][2] that brings in Penrose as a dependency.

The Penrose repository has several up to date examples of what a typical
`main.rs` ends up looking like and there is a guide on how to go from installing
rust to running Penrose as your window manager located [here][3]
<br><br>


## Does Penrose support Wayland as a back end?

Short answer: no.

Long answer:

Wayland merges the concept of the window manager with the that of the
compositor, which results in significantly more work (which I'm not planning on
doing given that I'm perfectly happy with X11 as a back end). The internal APIs
of Penrose only expect to be managing window positioning and workspaces (as far
as X is concerned) so while it _may_ be possibly to add Wayland support, it's
not a simple task. It is definitely something that would be interesting to look
into in the future but it's not a high priority for me personally as I am
perfectly happy running X11 for now.
<br><br>


## Where's the eye candy?

Short answer: there isn't any.

Long answer:

Penrose is, first and foremost, designed with simplicity, speed and stability in
mind. This means that the default, out of the box offering is pretty minimal.
I'm a big fan of the [unix philosophy][4] and with that in mind, Penrose largely
restricts its core functionality to managing your windows. Decorations and
animation are not first class citizens but can be added through extensions and
user code if desired.
<br><br>


## Are you accepting contributions for open issues?

Short answer: please discuss on the issue in question

Long answer:

Typically issues in the GitHub issue tracker are already being worked on or are
blocked for some particular reason that should be clear from the issue. If you
would like to work on an open issue that looks to be stalled please add a
comment to the issue in question registering your interest.

If you would like to raise a bug report or make a feature request then please
open a new issue and get confirmation that the change / approach to the fix is
somethat that is likely to be accepted before starting work.
<br><br>


### Can I raise a Pull Request adding a shiny new feature?

Short answer: please raise an issue first to discuss what it is you want to add.

Long answer:

No really, please make sure to raise an issue in GitHub _before_ raising a pull
request in the repo. I'm very happy to accept contributions for both bug fixes
and new functionality but (like most open source maintainers) I do not have time
to review pull requests that have had no prior discussion before being raised.
If there are any issues with the approach being taken (or breaking changes / conflicts
with ongoing work) it can end up with a reasonable amount of back and forth as
changes are requested and made.

Put simply, it's a far better experience for me as a maintainer and you as a
contributor to get a thumbs up on an approach before spending time on the implementation!
<br><br>


## Can you add 'feature X' from this other window manager?

Short answer: probably not as a core feature, but feel free to raise an issue to discuss it.

Long answer:

I started `penrose` because 1) I like hacking on stuff and it seemed like a fun
idea and 2) I was dissatisfied with the feature sets offered by other window
managers. Some had everything I wanted, but came with things that I really
didn't like, while others felt like they were missing features. `penrose` has
been written to be a base layer that you can build from to write a window
manager that works how you want it to: this means that there is a small set of
opinionated, core functionality and then a variety of ways to extend this. There
are likely a few pieces of functionality that I have missed that _can_ be added
to core without disrupting what is already there, but most "missing" features
from other window managers are missing on purpose. If you would like to add them
as an extension, please see the contribution guidelines above.

One important category of functionality that will not be added to the core of the
`penrose` crate itself is any sort of helper program or additional scripts that
aim to wrap `penrose` and make it look like a stand alone binary.

`penrose` is (as clearly stated in the README) a library, _not_ a binary.

If writing your own crate and compiling and installing the resulting binary is
not something you want to manage and maintain, then `penrose` is not for you.


  [0]: https://github.com/sminez/penrose
  [1]: https://crates.io/crates/penrose
  [2]: https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html
  [3]: https://github.com/sminez/penrose/blob/develop/docs/getting_started.md
  [4]: https://en.wikipedia.org/wiki/Unix_philosophy
