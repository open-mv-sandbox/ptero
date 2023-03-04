//! Stewart is a modular, flexible, and high-performance actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod addr;
pub mod dynamic;
mod system;
pub mod utils;

use tracing::{span, Level, Span};

pub use self::{
    actor::{Actor, AfterProcess, AfterReduce, Protocol},
    addr::{ActorAddr, ActorId},
    dynamic::{AnyActor, AnyMessage},
    system::System,
};
pub use anyhow::Error;
pub use stewart_derive::{Factory, Protocol};

/// Re-exported for macro generation.
pub use tracing;

pub trait Factory {
    /// Create a tracing span to be used in log messages.
    fn create_span(&self) -> Span {
        span!(Level::INFO, "unknown")
    }

    /// Consume the factory and start the actor.
    fn start(self: Box<Self>, system: &mut System, id: ActorId)
        -> Result<Box<dyn AnyActor>, Error>;
}
