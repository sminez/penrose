# Layouts

The built in layout functionality is primarily focused around giving a default
experience that is useful out of the box. With that in mind, things are restricted
to a couple of simple layouts that showcase the message handling capabilities of
the `Layout` trait, the associated `Messages` and a couple of `Transformers` that
combine nicely to give your windows a little bit of breathing room.


## Layouts
### Monocle

```
+-----------------------+
|                       |
|                       |
|                       |
|                       |
|                       |
+-----------------------+
```

The monocle layout is lifted directly from `dwm` as what is possibly the simplest
possible layout: the currently focused window gets the full available screen
space and everything else is hidden.

> **NOTE**: This is not the same thing as making a window fullscreen. With the
> monocle layout you will still see the effect of any `LayoutTransformers` that
> have been applied which may reduce the space available for the window.

### Main and Stack

```
+--------------+--------+
|              |        |
|              |        |
|              +--------+
|              |        |
|              |        |
+--------------+--------+
```

The default and primary layout for penrose is the `MainAndStack` which is a slight
generalisation of the default `tiled` layout from xmonad. There are several ways
to set it up but the common theme is the idea of a "main" area and stack (or
secondary) area that contains the windows that are not the current focus of what
you are doing. The number of windows allowed in the main area can be changed using
messages as can the proportions of the screen assigned to each area.

As you might expect you have the choice of whether the main area is on the left,
right, top or bottom of the screen. There are also a couple of `Messages` that can
be sent to switch between the different behaviours if you want to modify a single
layout rather than register several different ones.

### Centered Main

```
+-----------+-----------+
|           |           |
|           |           |
+-----------+-----------+
|                       |
|                       |
+-----------+-----------+
|           |           |
|           |           |
+-----------+-----------+
```

There is also a modified version of the `MainAndStack` layout called `CenteredMain`
which provides two secondary areas, one either side of the main area. As with its
counterpart, you can rotate between having the secondary areas to the side or above
and below the main area by sending a `Rotate Message`

### Grid
```
+-------+-------+-------+
|       |       |       |
|       |       |       |
+-------+-------+-------+
|       |       |       |
|       |       |       |
+-------+-------+-------+
|       |       |       |
|       |       |       |
+-------+-------+-------+
```

The `Grid` layout will tile windows in the smallest **nxn** grid that can hold the
number of windows present on the workspace.

Please be aware that if there are not a square number of windows to be tiled, this
layout will leave gaps:
```
+-------+-------+-------+
|       |       |       |
|       |       |       |
+-------+-------+-------+
|       |       |       |
|       |       |       |
+-------+-------+-------+
|       |       |
|       |       |
+-------+-------+
```


## Messages

As mentioned above, there are a handful of built in messages that work with the
`MainAndStack` layout which are also generally applicable to other layouts with a
similar sort of set up. The `IncMain`, `ExpandMain` and `ShrinkMain` messages should
be relevant for any layout that emphasises some clients over others. The `Rotate` and
`Mirror` messages can be used if a single layout supports rotational and reflective
symmetry (or if pairs of layouts can be mapped to one another).

The `UnwrapTransformer` message is tied to the `LayoutTransformer` trait as a way of
removing a layout transformer from the underlying layout. Nothing needs to be done
to support this message as it is handled by the `LayoutTransformer` trait itself.


## Transformers

To showcase a couple of simple things that are possible with `LayoutTransformers`, there
is are the `ReflectHorizontal` and `ReflectVertical` transformers which do pretty much
what you would expect. To support the built in status bar there is also a `ReserveTop`
transformer that can be used to prevent layouts from positioning windows over a status
bar, and finally there is the `Gaps` transformer because (lets face it) most of us like
at least a _little_ bit of space between our windows.
