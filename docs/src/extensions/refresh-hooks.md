# Refresh Hooks

Refresh hooks (like startup hooks) are added to your window manager as an implementation of the [StateHook][0] trait.
They are run at the end of the [modify_and_refresh][1] method of the `XConnExt` trait each time the internal state of
the window manager is refreshed and rendered to the X server. This is one of the more general purpose hooks available
for you to make use of and can be used to run code any time something changes in the internal state of your window
manager.

> **NOTE**: Xmonad refers to this as a "Log Hook" which I find a little confusing. The name comes from the fact that
> one of the main use cases is to log the internal state of the window manager in order to update a status bar, which
> makes sense but I prefer naming the hooks for where they are called in the event handling flow.

As with the other hooks, there is a [compose_or_set][2] method on `Config` for adding Refresh Hooks into you existing
`Config` struct.

  [0]: https://sminez.github.io/penrose/rustdoc/penrose/core/hooks/trait.StateHook.html
  [1]: https://sminez.github.io/penrose/rustdoc/penrose/x/trait.XConnExt.html#method.modify_and_refresh
  [2]: https://sminez.github.io/penrose/rustdoc/penrose/core/struct.Config.html#method.compose_or_set_refresh_hook
