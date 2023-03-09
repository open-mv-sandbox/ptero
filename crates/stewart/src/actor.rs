use anyhow::Error;
use family::Family;

use crate::System;

/// Active actor interface.
///
/// TODO: As of right now, actors are split up into `reduce` and `process`, but some actors only
/// use one of the two. Maybe move the queue system to an extension?
pub trait Actor {
    type Family: Family;

    /// Handle a message in-place.
    ///
    /// In most cases, you should store the message as appropriate until processing. This lets
    /// your actor handle messages in bulk, which is generally better for performance. As well,
    /// processing the message here means a higher chance of failure, as the sending actor is not
    /// available itself for handling during `reduce`.
    ///
    /// In some cases however, it's beneficial to immediately process the message. This prevents
    /// it from being penalized by being delayed by a queue round-trip. Especially when all your
    /// actor does is relay a message to another actor or system, this may be beneficial.
    fn reduce(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error>;

    /// Process previously reduced messages.
    fn process(&mut self, _system: &mut System) -> Result<After, Error> {
        Ok(After::Nothing)
    }
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
