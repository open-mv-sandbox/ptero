 # Stewart

> 🚧 This will become the stewart book, currently it just contains quick design notes to be worked out into full documentation as stewart is being built, and the notes are actually implemented.

## Actors

Actors are recursive, they can start and own other actors. Through this, an actor can manage a group of child actors that perform various sub-tasks. When the parent actor is stopped, its child actors are also stopped. This automatically cleans up resources.

### Lightweight

Actors are very lightweight, and can be used on a granular level. A few examples of lightweight actors are:

- A helper actor that translates messages from another system, into the messages of the parent actor.
- A public API actor that exposes a limited interface of its parent actor.

## Staying Local and Actor Stealing

Stewart's execution model is built around the idea that keeping related systems local, machine-local or even core-local, is generally better for performance when possible. This avoids having to unnecessarily transfer data.

For example, when keeping actors entirely core-local, they never have to send their inner data between cores, keeping it in core-local memory cache. While, if you kept jumping an actor between cores its data would need to continually be sent between cores. This assumption similarly extends at a larger scale. Executing something on the same machine is faster than sending it between machines.

Based on this idea, stewart is built around "actor stealing". Execution engines will track the actors they currently own, and if they have spare resources to spend they'll "steal" actors from other execution engines, if it would be a good idea to do so.
