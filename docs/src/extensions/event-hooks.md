# Event Hooks

[EventHooks][0] run before each event from the X server is processed, allowing you to provide your
own custom handling of events. You are free to run whatever code you want in response to events
and you are also able to decide whether or not the built-in event handling should run after you
are done: if you return `Ok(true)` from your hook then the processing will continue, if you return
`Ok(false)` then it will stop.

If you _do_ decide to skip default handling you should check carefully what it is that you are
skipping. The main event handling logic can be found [here][1] in the core module.

As with the other hooks, there is a [compose_or_set][2] method on `Config` to help you combine
multiple event hooks together without accidentally overwriting anything along the way.

> **NOTE**: EventHooks are run _in order_ and the first hook to say that no further processing
> should take place will short circuit any remaining composed event hooks and the default handling!


  [0]: https://sminez.github.io/penrose/rustdoc/penrose/core/hooks/trait.EventHook.html
  [1]: https://github.com/sminez/penrose/blob/develop/src/core/handle.rs
  [2]: https://sminez.github.io/penrose/rustdoc/penrose/core/struct.Config.html#method.compose_or_set_event_hook
  
