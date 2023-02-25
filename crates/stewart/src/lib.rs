//! A minimal modular actor interface.
//!
//! Stewart is built to allow for runtimes that distribute execution on both native and web
//! targets, and communicate with various async executors, even within the same process.
//!
//! This is a reference documentation for stewart, for more detailed explanation on stewart's
//! design philosophy, read the stewart book.

mod actor;
mod context;

use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use anyhow::Error;

pub use self::{
    actor::{Actor, AnyActor, Next},
    context::Context,
};
pub use stewart_derive::Factory;

/// Opaque target address of an actor.
pub struct Address<M> {
    address: usize,
    _m: PhantomData<AtomicPtr<M>>,
}

impl<M> Address<M> {
    pub fn from_raw(address: usize) -> Self {
        Self {
            address,
            _m: PhantomData,
        }
    }
}

impl<M> Clone for Address<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for Address<M> {}

/// Instructions for creating an actor on a runtime locally.
pub trait Factory {
    fn start(
        self: Box<Self>,
        ctx: &dyn Context,
        address: usize,
    ) -> Result<Box<dyn AnyActor>, Error>;
}
