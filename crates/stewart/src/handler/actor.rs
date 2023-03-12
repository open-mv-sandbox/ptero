use anyhow::Error;
use family::{utils::FamilyT, Family};

use crate::{After, System};

/// Message handling interface.
pub trait Actor {
    type Family: Family;

    /// Handle a message in-place.
    fn handle(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error>;
}

/// Convenience `Handler` specialization that operates on messages with a static lifetime.
///
/// See `family::utils` module for more information.
pub trait ActorT {
    type Message: 'static;

    /// Handle a message in-place.
    fn handle(&mut self, system: &mut System, message: Self::Message) -> Result<After, Error>;
}

impl<A> Actor for A
where
    A: ActorT,
{
    type Family = FamilyT<<Self as ActorT>::Message>;

    fn handle(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error> {
        self.handle(system, message.0)
    }
}
