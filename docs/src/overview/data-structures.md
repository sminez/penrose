<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Data Structures

As mentioned in [Pure Code vs X Code][0], there are a number of `pure` data structures that penrose makes use of
in order to manage the internal state of the window manager. We wont get too much into the details of all of the
various methods associated with each data structure: for that it's best to read the docs on docs.rs. Instead,
we'll take a quick look at what each data structure does and how you can make use of it when writing your own
penrose based window manager.

Most of the data structures outlines below are some form of [zipper][1] (or some meta-data wrapped around a zipper).
If the Wikipedia page all looks a bit "computer science-y" to you then you can get by pretty well by thinking of
a zipper as collection type (like a list or a tree) that has an added concept of "focus" (that is, "the element
of the collection we are currently looking at"). There is a really nice article about [the use of zippers in Xmonad][2]
which is worth a read if you have the time. It covers the starting point for the use of zippers in penrose and
also shows where all of the names come from(!) Penrose takes the idea a little further than what is seen in
Xmonad in order to provide what I think is a nicer API to work with (but I'll let you be the judge of that).

First up, the arguably incorrectly named "Stack".


### Stacks

So called because it (primarily) represents the X "window stack" for the current screen you are looking at. Getting
technical for a minute, a `Stack` is a zipper over a [doubly-linked list][3] that has a couple of nice properties
that help to simplify how a lot of the rest of the code in penrose is written:
  1. A Stack is _never_ empty (there is always at least the focused element)
  2. Operations that manipulate which element is focused do not alter the _order_ of the elements themselves.
  3. Operations that work with the focused element are `O(1)`

You can think of a `Stack` as simply being a normal linked list with a flag on one of the elements to indicate
where the focus point currently sits. (The actual implementation is a little different in order to make things
nicer to work with but the idea itself is fine).

Penrose makes use of `Stacks` for anything that we want to track focus for. Specifically, we use them for tracking:
  - windows assigned to each workspace
  - the layouts in use on each workspace
  - workspaces assigned to a each screen

The operations available on `Stacks` are pretty much what you'd expect: you can treat them like collections (map,
filter, re-order the elements, iterate, etc) and you can move the focus point around.


### Workspaces

Up next after `Stacks` is `Workspaces`. You can think of a workspace as a wrapper around a given window stack that
helps penrose know how to locate the given stack of clients and how (and when) to position them on the screen.
Rust type wise, a workspace look like this (the fields on a real `Workspace` aren't public but we can ignore that for now):
```rust
pub struct Workspace {
    id: usize,
    tag: String,
    layouts: Stack<Layout>,
    stack: Option<Stack<Xid>>,
}
```

The `id` and `tag` fields are used to identify workspaces within the larger pure state: useful, but not particularly
interesting. The client `Stack` itself is wrapped in an Option because (like we mentioned above) there is no such
thing as an empty `Stack`, so a `Workspace` with no windows has `None`. Running operations on the stack contained in
a given workspace is possible from the top level of the pure state (which we'll cover in a bit).

The `layouts` field contains all of the possible [Layout][4] algorithms available for positioning windows on this
workspace. There must be at least one layout available (so no `Option<Stack>` here) and the currently focused layout
in the stack is the one that will be used to position windows when this workspace is placed on a given screen.

Speaking of which...


### Screens

If you thought a `Workspace` was pretty much "a window Stack with a fancy hat", then a `Screen` is "a Workspace in a
box".

A 2D box to be precise.

For the purposes of our pure state, all we care about when it comes to the physical screens we have to play with are:
  - which screen we're talking about
  - the dimensions of the screen
  - the workspace that is currently active

Each screen pairs a `Workspace` with an ID (`0..n` in the order that they are returned to us by the X server) and a
`Rect` to denote the size and relative position of each screen in pixels. Workspaces can be moved between screens,
clients can be moved between workspaces.

Lovely.


#### Rect(angles)

Both screens and the windows that sit within them are described using rectangles. Each `Rect` is simply the `(x, y)`
coordinates of its top left corner along with its width and height. Not _massively_ exciting on its own but it's
worth taking a look at the docs on the `Rect` struct to see what methods are available for slicing, dicing, positioning
and comparing Rects while you write your custom Layout algorithms and extensions.


### The StackSet

And last but by no means least, we have the `StackSet`. It's a _little_ "set-y" when you break it down so that's what
we're going for name wise until someone gives me something better (it's definitely a lot _more_ like a set than the
original from Xmonad in my opinion but we'll get to that in a second).

Ignoring several book-keeping fields which we maintain for quality of life purposes, the Rust type looks something like
this:
```rust
struct StackSet {
    screens: Stack<Screen>,
    hidden: LinkedList<Workspace>,
    // and some book-keeping...
}
```

I'm not quite sure how best to describe what's going on here in terms of Zippers as it's a _little_ bit of an abuse of
the concept but, if you squint hard enough, what you're looking at is pretty much a "Stack of Stacks". Albeit with a
healthy sprinkling of meta-data throughout and the fact that for the unfocused elements we don't care about their order
(hence the [set][5] based name).

If you think back to what we said a `Zipper` was, we said we had some collection of elements along with the idea of there
being a "focus point" that picks out an element from that collection. For the `StackSet`, the collection is a set of
`Workpsaces`, and the "focus" is actually a `Stack` of `Screens` and their _associated_ `Workspaces`.

...still with me?

If you think about what we care about when managing windows, we can break things down into the following:
  - The windows we are managing (`Stacks`)
  - The workspaces those windows are assigned to (`Workspaces`)
  - The screens those workspaces are shown on (`Screens`)
  - The workspaces that are currently hidden from view (more `Workspaces`)

For the workspaces that are visible, we move them in and out of the available screens as needed and we maintain the
currently focused screen which is where the X input focus currently lies. For the hidden workspaces we don't really care
about what order they are in (we can't see them) so we use a LinkedList to store anything not currently allocated to a
screen.

> We _could_ use a `HashSet` but then we'd need Workspaces to be hashable and it doesn't actually buy us much in terms
> of the API we end up with.

Having the focused "element" be another level of wrapping around _multiple_ element from the collection really pushes
the definiton of a Zipper I suspect but it works pretty nicely all things considered. We can then fully manage the
on screen position and stack position of each window and manipulate groups of windows based on the workspace they are
part of.

Nice.


### And that's it!

Admittedly, "it" is a rather large set of methods on a `StackSet` but it gives you a rich, zipper based API for manipulating
your windows which handles all of the focus book-keeping for you. To really understand everything that is possible with
the API it is best to dive into the docs.rs docs and try things out for yourself. The _real_ structs are generic rather
than having to contain `Xids` as shown in the pseudo-code above so feel free to pull in penrose as a dependency and start
having a play with them to see what is possible!

The tests suites are another good place to take a look at how things work without getting too tied up in the specific use
cases penrose has for things.

Speaking of specifics, lets take a look at how to actually do useful things with your window manager: up next we're covering
layouts.


  [0]: ./pure-vs-x.md
  [1]: https://en.wikipedia.org/wiki/Zipper_(data_structure)
  [2]: https://donsbot.com/2007/05/17/roll-your-own-window-manager-tracking-focus-with-a-zipper/
  [3]: https://doc.rust-lang.org/std/collections/struct.LinkedList.html
  [4]: ./layouts.md
  [5]: https://en.wikipedia.org/wiki/Set_(mathematics)
