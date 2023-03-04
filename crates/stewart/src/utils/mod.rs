//! Small convenience utilities.

use anyhow::{bail, Error};
use family::{Family, StaticFamily};

use crate::{Actor, ActorAddr, AfterProcess, AfterReduce, System};

/// Convenience actor specialization that operates on messages with a static lifetime.
///
/// Automatically implements `Actor`, wrapping `Message` with `StaticFamily`.
/// Workaround for a lack of HKT support in Rust.
pub trait StaticActor {
    type Message: 'static;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce(&mut self, message: Self::Message) -> Result<AfterReduce, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

impl<A> Actor for A
where
    A: StaticActor,
{
    type Family = StaticFamily<<Self as StaticActor>::Message>;

    fn reduce<'a>(
        &mut self,
        message: <Self::Family as Family>::Member<'a>,
    ) -> Result<AfterReduce, Error> {
        self.reduce(message)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        self.process(system)
    }
}

/// Convenience alias for addresses of static actors.
pub type ActorAddrS<T> = ActorAddr<StaticFamily<T>>;

/// Should-be-unreachable placeholder actor.
///
/// Impossible to reduce, will raise an error if processed.
/// This actor can be used as a cheap placeholder when you need an actor in a slot that you'll
/// later replace with a 'real' actor.
pub struct UnreachableActor;

impl StaticActor for UnreachableActor {
    type Message = Unreachable;

    fn reduce<'a>(&mut self, _message: Unreachable) -> Result<AfterReduce, Error> {
        unreachable!()
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        bail!("attempted to process UnreachableActor")
    }
}

/// Impossible to create type.
pub enum Unreachable {}
