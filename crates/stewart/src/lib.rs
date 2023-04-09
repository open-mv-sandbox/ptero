//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod actor_tree;
mod node;
mod slot;
mod system;

pub use self::{
    actor::{Actor, After},
    actor_tree::{CreateActorError, Id, StartActorError},
    node::Options,
    system::{Addr, System},
};

/// Helper newtype for passing the parent when creating an actor.
///
/// If an actor has a parent, it will be stopped when its parent is stopped.
///
/// Implements `From` with `Id`, for easy conversion.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Parent(pub Option<Id>);

impl Parent {
    pub fn root() -> Self {
        Self(None)
    }
}

impl From<Id> for Parent {
    fn from(value: Id) -> Self {
        Parent(Some(value))
    }
}

// TODO: Add 'context' for running operations 'in the context of' something?
