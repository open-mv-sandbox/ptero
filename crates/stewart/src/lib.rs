//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod addr;
mod info;
mod system;

pub use self::{
    actor::{Actor, After},
    addr::Addr,
    info::{Id, Info},
    system::{CreateActorError, StartActorError, System},
};
