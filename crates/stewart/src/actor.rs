use anyhow::Error;

use crate::{Id, Messages, System};

/// Message handling interface.
pub trait Actor: 'static {
    type Message;

    /// Perform a processing step.
    fn process(
        &mut self,
        system: &mut System,
        id: Id,
        messages: &mut Messages<Self::Message>,
    ) -> Result<After, Error>;
}

/// The operation to perform with the actor after a message was handled.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum After {
    /// Continue running the actor.
    Continue,
    /// Stop the actor and remove it from the system.
    Stop,
}
