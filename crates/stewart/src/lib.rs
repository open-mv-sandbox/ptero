//! Stewart is a modular, flexible, and high-performance actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod addr;
pub mod dynamic;
mod factory;
mod system;
pub mod utils;

pub use self::{
    actor::{Actor, AfterProcess, AfterReduce, Protocol},
    addr::{ActorAddr, ActorId},
    dynamic::{AnyActor, AnyMessage},
    factory::{Factory, Start},
    system::System,
};
pub use anyhow::Error;
pub use stewart_derive::{Factory, Protocol};

/// Re-exported for macro generation.
pub use tracing;
