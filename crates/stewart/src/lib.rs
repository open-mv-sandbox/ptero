//! A minimal modular actor interface.
//!
//! Stewart is built to allow for runtimes that distribute execution on both native and web
//! targets, and communicate with various async executors, even within the same process.
//!
//! This is a reference documentation for stewart, for more detailed explanation on stewart's
//! design philosophy, read the stewart book.

mod actor;
mod address;
mod context;
mod factory;

pub use self::{
    actor::{Actor, Next},
    address::Address,
    context::Context,
    factory::{AnyActor, Factory},
};
pub use stewart_derive::Factory;
