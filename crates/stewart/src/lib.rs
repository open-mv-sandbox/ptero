//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

pub mod handler;
mod info;
pub mod schedule;
mod system;

pub use self::{
    info::{Id, Info},
    system::{BorrowError, CreateActorError, StartActorError, System},
};

/// The operation to perform with the actor after performing an operation on it.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum After {
    /// Do nothing, no changes are made.
    Nothing,
    /// Stop the actor and remove it from the system.
    Stop,
}
