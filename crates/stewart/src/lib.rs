//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod actor_tree;
mod info;
mod node;
mod slot;
mod system;

pub use self::{
    actor::{Actor, After},
    actor_tree::{CreateActorError, StartActorError},
    info::{Addr, Id, Info},
    node::{Options, StoreError, TakeError},
    system::System,
};
