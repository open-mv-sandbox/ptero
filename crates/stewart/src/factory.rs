use anyhow::Error;
use tracing::{span, Level, Span};

use crate::{Actor, ActorAddr, ActorId, AnyActor, System};

pub trait Factory {
    /// Create a tracing span to be used in log messages.
    fn create_span(&self) -> Span {
        span!(Level::INFO, "unknown")
    }

    /// Consume the factory and start the actor.
    fn start(self: Box<Self>, system: &mut System, id: ActorId)
        -> Result<Box<dyn AnyActor>, Error>;
}

/// Starting interface for actors.
///
/// This trait is optional, and mainly used by the `Factory` trait's derive macro.
pub trait Start: Actor + Sized {
    type Data;

    fn start(
        system: &mut System,
        addr: ActorAddr<<Self as Actor>::Protocol>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}
