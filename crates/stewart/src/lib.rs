//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod dynamic;
mod family;
mod info;
mod system;

pub use self::{
    actor::{Actor, AfterProcess, AfterReduce},
    family::{ActorT, AddrT},
    info::{Addr, Id, Info},
    system::{BorrowError, CreateActorError, ProcessError, StartActorError, System},
};
