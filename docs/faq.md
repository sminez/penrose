<image width="60px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Penrose FAQs
<br>

## How do I install Penrose?
You don't: Penrose is a library that you use to write your own window manager.
Take a look at the [getting started guide][3] for details of how to use Penrose
as a library.
<br><br>

## Where can I view the Penrose source code?

Penrose is developed openly on [GitHub][0] and published to [crates.io][1]
periodically as new features are added. The `develop` branch always has the
latest code and is what I use for running Penrose on my personal laptop. It
is not advised that you pin your use of Penrose to the GitHub `develop` branch
as a typically end user however: breaking changes (and weird and wonderful bugs)
are highly likely. You have been warned!
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

Short answer: not currently.

Long answer: Wayland merges the concept of the window manager with the that of the
compositor, which results in significantly more work (which I'm not planning on
doing given that I'm perfectly happy with X11 as a back end). The internal APIs
of Penrose only expect to be managing window positioning and workspaces (as far
as X is concerned) so while it _may_ be possibly to add Wayland support, it's
not a simple task. It is definitely something that I'd be interested in looking
into in the future but it's not a high priority for me personally as I am
perfectly happy running X11 for now.
<br><br>


## Where's the eye candy?

Penrose is, first and foremost, designed with simplicity, speed and stability in
mind. This means that the default, out of the box offering is pretty minimal.
I'm a big fan of the [unix philosophy][4] and with that in mind, Penrose largely
restricts its core functionality to managing your windows. Decorations and
animation are not first class citizens but can be added through extensions and
user code if desired.
<br><br>

## Are you accepting contributions?

### Labelled 'good first issue'

Short answer: yes!

Long answer:

Any issues marked with `good first issue` should have sufficient detail to
allow you to get started working with the current state of develop. Please
comment on the issue asking to pick it up before starting work and I can assign
it to you. Depending on how old the issue is, it may need to be updated before
you can get going.

### Contrib & alternate trait impls

Short answer: yes, but please raise an "enhancement" issue explaining the idea.

Long answer:

Please raise an issue first outlining what it is you would like to add (e.g. a
new layout function, a hook, [an entire new backend][5] (!), documentation,
examples) and make sure that you are working from latest `develop`. Depending on
how much free time I currently have, I may be making large changes or additions
to the codebase at any given time and you will want to make sure that you are
following any work that is being done to the core of `penrose` itself.

### Picking up issues in Core, Draw or existing trait impls

Short answer: ask on the issue if it is unassigned.

Long answer:

I tend to use the GitHub issue system as a way of communicating the state of
development on `penrose` to those who are interested and also as a bit of a
project diary. It helps to document how and why things were done (in addition to
the git log) in a way that is easily accesible without having to hunt through
commit messages. I try to make sure that issues that are more my personal `TODO`
list items are assigned to me in GitHub, but please ask on any issue you want to
pick up before starting work: I can be forgetful sometimes and may already be
working on it!

<br>

## Can you add 'feature X' from this other window manager?

Short answer: probably not as a core feature, but raise an issue to discuss.

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

One important category of functionality that will not be added to the `penrose`
crate itself is any sort of helper program or additional scripts that aim to
wrap `penrose` and make it look like a stand alone binary.

`penrose` is (as clearly stated in the README) a library, _not_ a binary.

If writing your own crate and compiling and installing the resulting binary is
not something you want to manage and maintain, then `penrose` is not for you.


  [0]: https://github.com/sminez/penrose
  [1]: https://crates.io/crates/penrose
  [2]: https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html
  [3]: https://github.com/sminez/penrose/blob/develop/docs/getting_started.md
  [4]: https://en.wikipedia.org/wiki/Unix_philosophy
  [5]: https://github.com/sminez/penrose/issues/104
