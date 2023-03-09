use anyhow::Error;
use family::{utils::FamilyT, Family};

use crate::{After, System};

/// Message handling interface.
pub trait Handler {
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
pub trait HandlerT {
    type Message: 'static;

    /// Handle a message in-place.
    fn handle(&mut self, system: &mut System, message: Self::Message) -> Result<After, Error>;
}

impl<A> Handler for A
where
    A: HandlerT,
{
    type Family = FamilyT<<Self as HandlerT>::Message>;

    fn handle(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error> {
        self.handle(system, message.0)
    }
}
