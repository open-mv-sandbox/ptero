//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod actors;
mod addr;
mod info;
mod system;

pub use self::{
    actor::{Actor, After},
    actors::CreateActorError,
    addr::Addr,
    info::{Id, Info},
    system::{StartActorError, System},
};
