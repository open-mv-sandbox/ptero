use anyhow::{bail, Error};
use void::Void;

use crate::{utils::ActorT, AfterProcess, AfterReduce, System};

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
