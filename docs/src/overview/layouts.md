<image width="50px" src="https://raw.githubusercontent.com/sminez/penrose/develop/icon.svg" align="left"></image>
# Layouts

Layouts are (lets face it) a large part of why people use a dynamic tiling window manager in the first place.
You want to automatically manage your windows in a way that either lets you get on with what you're doing, or
looks fun and interesting!

For penrose, layouts are implemented using a trait that lets you specify how the layout should be applied and
manage any additional state you might need. They also support custom messages being sent to modify their
behaviour and update that state: another shamelessly re-used idea from Xmonad. You may be starting to spot a
pattern here...


### Taking a look at the Layout trait

Other than a few pieces of housekeeping (providing a string name to be used to identify the layout and some
plumbing to help with dynamic typing) the `Layout` trait is primarily several methods that give you (the
implementer) some flexability in how you want to approach positioning your windows and how what level of
customisation you want to give the user while the window manager is running:

```rust
pub trait Layout {
    fn name(&self) -> String;
    fn boxed_clone(&self) -> Box<dyn Layout>;

    fn layout_workspace(
        &mut self,
        tag: &str,
        stack: &Option<Stack<Xid>>,
        r: Rect
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>);

    fn layout(
        &mut self,
        s: &Stack<Xid>,
        r: Rect
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>);
    
    fn layout_empty(
        &mut self,
        r: Rect
    ) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>);

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>>;
}
```

On the "laying out windows" front (you know, the main one) you have three choices:
  - Specify how to layout a possibly empty workspace based on the specific tag being laid out
  - Specify how to layout a given (non-empty) stack of clients for any workspace
  - Specify what to do when there are no clients present on the given workspace

Both `layout_workspace` and `layout_empty` have default implementations that should work in 99% of cases,
leaving you the job of writing `layout`: how a given screen `Rect` should be split up between a given
`Stack` of client windows. That said, if you _do_ want to specify how to layout particular workspaces or
give some custom logic that should run when a workspace is empty, both default implementations are of course
overridable.

> If you haven't read it already, it's worthwhile taking a look at the [data structures][0] section of this
> book to familiarise yourself with the types being discussed here!


### Writing a layout function

At it's core, a layout function is pretty simple: for a given region of screen real estate, assign sub-regions
to any number of the clients present on the workspace. There are no requirements to position _every_ client
and there are no requirements that clients do not overlap. There's just one key piece of information to bear
in mind:

> _The order that you return your positions in is the order that the windows will be stacked from top to bottom_.

If none of the `Rects` you return overlap then this doesn't matter all that much, but if you _do_ care about
stacking order, make sure to return your positions in order of top to bottom. Positions themselves are simply a
tuple of `(Xid, Rect)`. Any client window present in the provided `Stack` that you do not assign a position will
be unmapped from the screen.

As a simple example, here is the definition (in full) of the `Monocle` layout from the `builtin` module:
```rust
#[derive(Debug, Clone, Copy)]
pub struct Monocle;

impl Layout for Monocle {
    fn name(&self) -> String {
        "Mono".to_owned()
    }

    fn boxed_clone(&self) -> Box<dyn Layout> {
        Box::new(Monocle)
    }

    fn layout(&mut self, s: &Stack<Xid>, r: Rect) -> (Option<Box<dyn Layout>>, Vec<(Xid, Rect)>) {
        (None, vec![(s.focus, r)])
    }

    fn handle_message(&mut self, _: &Message) -> Option<Box<dyn Layout>> {
        None
    }
}
```

Pretty simple right? Admittedly, this is about as simple as you can make it (the focused window gets the
full screen and everything else gets unmapped) but the overall boilerplate is kept to a minimum, which
is nice.

> **NOTE**: The `builtin` module has some good examples of what a "real" layout looks like (not to dunk
> on `Monocle` but...come on). Why not take a look at `MainAndStack` as a starting point for how to write
> something a little more interesting?

But, I hear you cry (silently, through the internet) those `layout_*` methods don't just return a
`Vec<(Xid, Rect)>` do they? They also return an `Option<Box<dyn Layout>>`. What's up with that?

I'm so glad you asked.


#### Swapping things out for a new layout

Depending on how fancy you want to get with your layout behaviour, you might find yourself wanting to switch
things out to a new `Layout` implementation after you've positioned a stack of client windows for a particular
screen. Maybe you want to swap things out for a different layout depending on the number of clients, or the
screen size, or whether the width of the screen is a multiple of 7, or maybe you want the layout to change each
time it gets applied. Who knows! The point is, if you _do_ find yourself needing to swap things out this is a
way for you to do it.

In most cases you'll simply want to return `None` as the first value in the tuple being returned from layout
methods, but if you instead return `Some(new layout)`, penrose will swap out your current layout for the new
one.

If instead you just want to update some internal state in response to an explicit trigger, that's where `Messages`
come in.


### Handling messages

`Messages` are a way of sending dynamically typed data to your layouts in order to update their state. A message
can be [literally anything][1] so long as it implements the `IntoMessage` trait, which is as simple as:
```rust
impl IntoMessage for MyMessage {}
```

What any given message actually _does_ is entirely at the discression of the `Layout` that handles it. So far,
so vague...lets take a look at an example:
```rust
use penrose::core::layout::{IntoMessage, Layout, Message};

// First we define our message and implement the marker trait
struct SetFrobs(pub usize);
impl IntoMessage for SetFrobs {}

// Next we write our layout
struct MyLayout {
    frobs: usize,
}

impl Layout for MyLayout {
    // TODO: actually write the layout(!)

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        // If the Message is a 'SetFrobs' we'll do what it says on the tin...
        if let Some(&SetFrobs(frobs)) = m.downcast_ref() {
            self.frobs = frobs;
        }

        // ...and anything else we can just ignore

        None
    }
}
```

The `downcast_ref` method is the thing to pay attention to here: this is how we go from a `Message` (really just
a wrapper around the standard library `Any` trait) to a concrete type. Anything that implements `IntoMessage`
can be sent to our Layout so we do our own type checking to see if the message is something we care about. Messages
that we don't handle can safely be dropped on the floor (so don't worry about needing to exhaustively check all
possible message types).

The `Option<Box dyn Layout>` return type is the same idea as with the `layout_*` methods covered above: in response
to a message you can swap out to a new layout. Say hypothetically, there was a frob threshold above which things
got really awesome...
```rust
// A more AWESOME layout
struct MyAwesomeLayout {
    frobs: usize,
}

// Which has its own Layout implementation
impl Layout for MyAwesomeLayout {
    // ...
}

const AWESOMENESS_THRESHOLD: usize = 42;

// Now, we modify our impl for MyLayout to "level up" once we hit the threshold
impl Layout for MyLayout {
    // TODO: still need to write the layout at some point...

    fn handle_message(&mut self, m: &Message) -> Option<Box<dyn Layout>> {
        if let Some(&SetFrobs(frobs)) = m.downcast_ref() {
            if frobs > AWESOMENESS_THRESHOLD {
                // Things are getting awesome!
                return Some(Box::new(MyAwesomeLayout { frobs }));
            }

            // Still pretty cool, but not awesome yet...
            self.frobs = frobs;
        }

        None
    }
}
```

Nice!

That's all well and good if we have a bunch of our own layouts that we can write and swap between, but what if we
just want to _tweak_ an existing layout a bit? Well that's where we move over to the wonderful world of
`LayoutTransformers`.


### Layout transformers

This one is a bit of a rabbit hole...for now we'll cover the basics of what you can do with a transformer and leave
the details to the module docs themselves as there's quite a bit to cover!

`LayoutTransformer` is (surprise, surprise) another trait you can implement. It represents a wrapper around an inner
`Layout` which you (the author of the transformer) get to ~~lie to~~ help reach its full potential. The two main
things that a transformer can do are:
  - Modify the dimensions of the initial `Rect` being passed to the inner layout
  - Modify the positions returned by the inner layout before they are handed off for processing

So what does that let you do? Well for one thing, this is how gaps are implemented for any layout in penrose. The
`Gaps` transformer from the `builtin` module shrinks the size of the initial screen seen by the inner layout (to
give you an outer gap) and then shrinks the size of each window once the layout has run (to give you an inner gap).

For simple cases where you just want to modify the positions returned by an inner layout, there's a handy builtin
macro to generate a `LayoutTransformer` from a function:
```rust
use penrose::{pure::geometry::Rect, simple_transformer, Xid};

fn my_transformer(r: Rect, positions: Vec<(Xid, Rect)>) -> Vec<(Xid, Rect)> {
    // Write your transformation implementation here
}

simple_transformer!("MyTransform", MyTransformer, my_transformer);
```


  [0]: ./data-structures.md
  [1]: https://doc.rust-lang.org/std/any/trait.Any.html
