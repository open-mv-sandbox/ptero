use anyhow::Error;
use family::Family;

use crate::System;

/// Active actor interface.
pub trait Actor {
    type Family: Family;

    /// Handle a message in-place.
    fn handle(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error>;
}

/// The operation to take after the `reduce` or `process` step.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum After {
    /// Do nothing, no changes are made.
    Nothing,
    /// Stop the actor and remove it from the system.
    Stop,
}
