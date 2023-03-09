use anyhow::Error;
use family::{utils::FamilyT, Family};

use crate::{Actor, Addr, After, System};

/// Convenience actor specialization that operates on messages with a static lifetime.
///
/// Automatically implements `Actor`, wrapping `Message` with `FamilyT`.
/// See `family::utils` module for more information.
pub trait ActorT {
    type Message: 'static;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce(&mut self, system: &mut System, message: Self::Message) -> Result<After, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<After, Error>;
}

impl<A> Actor for A
where
    A: ActorT,
{
    type Family = FamilyT<<Self as ActorT>::Message>;

    fn reduce(
        &mut self,
        system: &mut System,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<After, Error> {
        self.reduce(system, message.0)
    }

    fn process(&mut self, system: &mut System) -> Result<After, Error> {
        self.process(system)
    }
}

/// Convenience alias for addresses of `ActorT`.
pub type AddrT<T> = Addr<FamilyT<T>>;
