# Manage Hooks

[ManageHooks][0] allow you to modify how a window is initially added to the window manager state when
it first appears. For example you might move the client to a specific workspace or position in the
stack, or you might mark it as floating in a certain position on the screen. Your hook will be called
after the window has been added into the internal state so the full set of APIs will be available for
you to make use of.

Again, as with the other hooks there is a [compose_or_set][2] method on `Config` to help you combine
multiple manage hooks together without accidentally overwriting anything along the way.


  [0]: https://sminez.github.io/penrose/rustdoc/penrose/core/hooks/trait.ManageHook.html
  [1]: https://sminez.github.io/penrose/rustdoc/penrose/core/struct.Config.html#method.compose_or_set_manage_hook
  
