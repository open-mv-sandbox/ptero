//! Stewart is a minimalist, high-performance, modular, and non-exclusive actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod actor_tree;
mod node;
mod slot;
mod system;

use anyhow::Error;
use thiserror::Error;

pub use self::{
    actor::{Actor, After},
    actor_tree::Id,
    node::Options,
    system::{Addr, Context, System},
};

#[derive(Error, Debug)]
pub enum AddrError {
    #[error("can't get addr of root")]
    Root,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CreateError {
    #[error("actor isn't pending to be started")]
    ParentDoesNotExist,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StartError {
    #[error("can't start root")]
    Root,
    #[error("actor already started")]
    ActorAlreadyStarted,
    #[error("internal error")]
    Internal(#[from] Error),
}
