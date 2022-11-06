# UI

Currently, penrose offers a single piece of built in UI via the [penrose_ui][0] crate:
a status bar. The bar is inspired by the `dwm` status bar and provides a simple API
for writing your own text based [widgets][1] for rendering to the screen.

In addition to the widgets described below there are a couple of debugging based widgets
which are useful when trying to diagnose issues with the window manager state but probably
_not_ something you want on your screen all the time. If you are interested in taking a
look at them they can be found [here][2]


## The Text widget

For building up simple widgets there is the [Text][3] widget which can be used to
provide most of the layout and re-render logic with an easy to use API. Any time the
contents of the widget are modified it will be re-rendered to the bar. On its own this
isn't particularly useful but you can add hooks to set the content in response to changes
in the window manager state (which we'll take a look at in the next section).

Text widgets are left justified by default but this can be switched to right justified if
desired. There is also the ability to specify that the widget is `greedy` which will cause
it to take up any available left over space once all other widgets have finished laying
out their contents. Personally I use this with the `ActiveWindowName` widget to take up
the middle of the status bar and act as a sort of active screen indicator .


## Built in widgets

### Workspaces

The [Workspaces][4] widget is the most complicated built in widget on offer. It checks the
currently available workspaces and several properties about each one:
  - The tag assigned to the workspace
  - Whether or not the workspace is focused (and on what screen)
  - If there are any windows visible on the workspace

From that it will generate a workspace listing with highlighting to indicate the current
state of your window manager. Workspaces with windows present are assigned a different
foreground color and focused workspaces are assigned a different background color. The
active workspace is indicated with its own highlight for visibility as well.


### RootWindowName

The [RootWindowName][5] widget is an idea lifted directly from dwm: any time the root window
name is updated it will re-render with its content set to the new name. The [xsetroot][6]
tool can be used to set the root window name to whatever string you like and typically
this is used by spawning a shell script that updates the root window name with system
stats on an interval:
```sh
# Set the root window name to the current date and time
$ xsetroot -name "$(date '+%F %R')"
```


### ActiveWindowName

In a similar way, [ActiveWindowName][7] will display the title of the currently focused
window. Given that there is less control over what the contents of this string will be,
this widget allows you to set a maximum character count after which the title is
truncated to `...`.

This widget will also only render on the active screen so it works well as a visual
indicator of which screen currently has focus.


### CurrentLayout

The [CurrentLayout][8] widget simply calls the `layout_name` method on the active workspace
each time the internal state is refreshed. Each `Layout` is free to specify whatever name it
choses so if you want to customise the text displayed by this widget you will need to write a
`LayoutTransformer` that intercepts the inner name and maps it to your preferred string
instead (or write a new widget that bakes that behaviour into the widget itself).


  [0]: https://sminez.github.io/penrose/rustdoc/penrose_ui/index.html
  [1]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/trait.Widget.html
  [2]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/debug/index.html
  [3]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/struct.Text.html
  [4]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/struct.Workspaces.html
  [5]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/struct.RootWindowName.html
  [6]: https://man.archlinux.org/man/xsetroot.1.en
  [7]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/struct.ActiveWindowName.html
  [8]: https://sminez.github.io/penrose/rustdoc/penrose_ui/bar/widgets/struct.CurrentLayout.html
