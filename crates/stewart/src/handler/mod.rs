//! Message dispatching and handling.

// TODO: Move back into core

mod actor;
mod sender;

pub use self::{
    actor::{Actor, ActorT},
    sender::{apply, Apply, Sender, SenderT},
};
