//! Stewart is a modular, flexible, and high-performance actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod addr;
mod dynamic;
mod factory;
mod system;
pub mod utils;

pub use self::{
    actor::{Actor, AfterProcess, AfterReduce, Start},
    addr::ActorAddr,
    system::{System, SystemError},
};
