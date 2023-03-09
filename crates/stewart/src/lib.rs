//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod family;
mod info;
mod sender;
mod system;

pub use self::{
    actor::{Actor, After},
    family::{ActorT, SenderT},
    info::{Id, Info},
    sender::Sender,
    system::{BorrowError, CreateActorError, StartActorError, System},
};
