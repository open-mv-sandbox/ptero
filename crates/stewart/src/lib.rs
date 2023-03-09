//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

pub mod handler;
mod info;
mod system;

pub use self::{
    info::{Id, Info},
    system::{BorrowError, CreateActorError, StartActorError, System},
};
