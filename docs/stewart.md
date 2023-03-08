# Stewart

Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.

Don't know what actors are or why you would want them? Skip ahead to the next section.

### Minimalist

Minimal core, minimal overhead, minimal boilerplate, minimal complexity. While stewart is full-featured through its modularity, its core remains small and focused. In fact, the core crate doesn't even provide an event loop! This makes it easy to use in a lot of cases, as it makes very few assumptions, and requires little work to add new actors.

### High-Performance

Stewart has minimal overhead, and is built around guiding you to improve your actors' performance. It isn't built with the assumption your actors' performance will be negligible compared to IO. This allows you to use it for speeding up CPU-bound operations too.

### Modular

While stewart is minimal, more complex features are provided through additional crates. This allows it to provide a broad range of features, without locking you into them if you don't need them. Additionally, you can replace parts entirely if they don't fit your use case.

### Non-Exclusive

Stewart does not assume you're buying into a larger 'stewart ecosystem'. It can be used with any event loop, async runtime, or even other actor systems.

## Why Actors

Actors are a programming pattern where "actors" receive "messages", and actors manage their own mutable state internally.

Typically, actors send messages to other actors to communicate in software based on the actor pattern, but the actor design pattern does not require your entire application to use actors.

Actors are useful for multiple reasons.

### Async-Await, Co-routines

Actors typically are seen as an alternative to "async-await", though they can be used together. In fact, modern async runtimes are very similar to actor systems. However, using actors over async-await has a few benefits.

Actors are simple, and do not particularly need any language features to work well like async-await does. This makes them easy to understand. Though some languages, like Erlang, do make actors a first-class language feature.

Actors make the 'suspending' of async-await and coroutines instead an explicit part of your actor's design, where your actor's mutable state is used instead of a suspended function's local scope.

When modeling an ongoing process rather than a linear series of steps, the design of actors especially makes them a much better fit than async-await. In the case of modeling a series of steps, async-await does work better, though there are patterns to improve this too. Additionally, you can build an async-await actor on top of an actor system.

### Multi-Threading

### Reliability

### Managing Mutable State

### Encapsulation
