# Startup Hooks

Startup hooks are run a single time after you call the [run][0] method on the `WindowManager` struct.
This takes before entering the main event loop but after all other setup has taken place. Any startup
actions you need to take that require the interaction with the X server or manipulating the window manager
state need to placed in here as a [StateHook][1] (completely custom code independent of the window manager
or X server can be run in your `main.rs` instead if you prefer).

The [compose_or_set_startup_hook][2] method on `Config` can be used to compose together multiple startup
hooks if you are making use of other extensions that also need to set one.

> **NOTE**: it is always best to use this method for setting additional hooks after you have created you
> initial `Config` struct in order to avoid accidentally replacing an existing hook!


  [0]: https://sminez.github.io/penrose/rustdoc/penrose/core/struct.WindowManager.html#method.run
  [1]: https://sminez.github.io/penrose/rustdoc/penrose/core/hooks/trait.StateHook.html
  [2]: https://sminez.github.io/penrose/rustdoc/penrose/core/struct.Config.html#method.compose_or_set_startup_hook
