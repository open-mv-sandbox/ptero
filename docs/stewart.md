# Stewart

Stewart is a modular, flexible, and high-performance actor system.

## Avoiding Unnecessary Overhead

Stewart's foundation is based on thread-local `System` instances. A `System` is a collection of running actors, local to that execution thread, and a queue of which actors to process next.

Using this design, execution is kept local to a CPU core where possible. This avoids needing synchronization primitives to pass data between CPU cores when it's not needed, and keeps it cache-local. Additionally, passing messages between actors on the same thread does not require the data to be owned inside the message. Actors can decide themselves during `reduce` what the most optimal way to store the data is until `process`.

This does not prevent actors from passing data between `System`s on different cores when needed, and stewart provides an additional library to make this easy. This also lets you tightly control what group of actors is on what thread, which can be useful if specific threads handle IO and need to be ready.

## Bring-Your-Own Event-Loop

Stewart's `System` execution engine doesn't implement a specific event loop. You can use it on existing event loops, such as `mio`, `winit`, Win32's `GetMessage`, or even browsers and web workers.
