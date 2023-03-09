use anyhow::Error;
use family::Family;

use crate::System;

/// Active message handler.
pub trait Actor {
    type Family: Family;

    /// Handle a message in-place.
    ///
    /// In most cases, you should store the message as appropriate until processing. This lets
    /// your actor handle messages in bulk, which is generally better for performance. As well,
    /// processing the message here means a higher chance of failure, as the sending actor is not
    /// available itself for handling during `reduce`.
    ///
    /// However, if you absolutely need to, you can process (and redirect) the message in-place
    /// here. In some cases this is better, such as if your actor's only task is to relay the
    /// message to another actor. This also does not require you to convert a borrowed message
    /// into owned, to queue it before re-sending it.
    fn reduce(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error>;

    /// Process previously reduced messages.
    fn process(&mut self, system: &mut System) -> Result<After, Error>;
}

/// The operation to take after the `reduce` or `process` step.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum After {
    /// Do nothing, no changes are made.
    Nothing,
    /// Queue the actor for processing.
    Process,
    /// Stop the actor and remove it from the system.
    Stop,
}
