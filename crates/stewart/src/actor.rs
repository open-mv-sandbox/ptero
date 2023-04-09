use anyhow::Error;

use crate::System;

/// Message handling interface.
pub trait Actor: 'static {
    type Message: 'static;

    /// Handle a message.
    ///
    /// TODO: Bulk operation?
    fn handle(&mut self, system: &mut System, message: Self::Message) -> Result<After, Error>;
}

/// The operation to perform with the actor after a message was handled.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum After {
    /// Continue running the actor.
    Continue,
    /// Stop the actor and remove it from the system.
    Stop,
}
