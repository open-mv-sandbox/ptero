//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod system;
mod tree;
mod world;

pub use self::{
    system::{State, System},
    tree::CreateError,
    world::{ActorId, Addr, SystemId, World},
};

// TODO: Remove anyhow from all public interfaces and add an InternalError type.
