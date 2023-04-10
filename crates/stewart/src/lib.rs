//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod actor_tree;
mod context;
mod node;
mod slot;
mod system;

pub use self::{
    actor::{Actor, ActorData, After},
    actor_tree::{CreateError, Id, StartError},
    context::Context,
    node::Options,
    system::{Addr, System},
};
