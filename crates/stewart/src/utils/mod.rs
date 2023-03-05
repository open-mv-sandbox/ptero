//! Small convenience utilities.

use anyhow::{bail, Error};
use family::{utils::FamilyT, Family};

use crate::{Actor, ActorAddr, AfterProcess, AfterReduce, System};

/// Convenience actor specialization that operates on messages with a static lifetime.
///
/// Automatically implements `Actor`, wrapping `Message` with `FamilyT`.
/// See `family::utils` module for more information.
pub trait ActorT {
    type Message: 'static;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce(&mut self, message: Self::Message) -> Result<AfterReduce, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

impl<A> Actor for A
where
    A: ActorT,
{
    type Family = FamilyT<<Self as ActorT>::Message>;

    fn reduce(
        &mut self,
        message: <Self::Family as Family>::Member<'_>,
    ) -> Result<AfterReduce, Error> {
        self.reduce(message.0)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        self.process(system)
    }
}

/// Convenience alias for addresses of `ActorT`.
pub type ActorAddrT<T> = ActorAddr<FamilyT<T>>;

/// Should-be-unreachable placeholder actor.
///
/// Impossible to reduce, will raise an error if processed.
/// This actor can be used as a cheap placeholder when you need an actor in a slot that you'll
/// later replace with a 'real' actor.
pub struct VoidActor;

impl ActorT for VoidActor {
    type Message = Void;

    fn reduce<'a>(&mut self, _message: Void) -> Result<AfterReduce, Error> {
        unreachable!()
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        bail!("attempted to process void actor")
    }
}

pub use void::Void;
