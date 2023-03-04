use anyhow::{bail, Error};

use crate::{Actor, AfterProcess, AfterReduce, System};

/// Should-be-unreachable placeholder actor.
///
/// Impossible to reduce, will raise an error if processed.
/// This actor can be used as a cheap placeholder when you need an actor in a slot that you'll
/// later replace with a 'real' actor.
pub struct UnreachableActor;

impl Actor for UnreachableActor {
    type Message = Unreachable;

    fn reduce<'a>(&mut self, _message: Unreachable) -> Result<AfterReduce, Error> {
        unreachable!()
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        bail!("attempted to process UnreachableActor")
    }
}

pub enum Unreachable {}
