use crate::{Actor, AfterReduce, Protocol};

/// Should-be-unreachable placeholder actor.
///
/// Impossible to reduce, will raise an error if processed.
/// This actor can be used as a cheap placeholder when you need an actor in a slot that you'll
/// later replace with a 'real' actor.
pub struct UnreachableActor;

impl Actor for UnreachableActor {
    type Protocol = Unreachable;

    fn reduce<'a>(&mut self, _message: Unreachable) -> AfterReduce {
        unreachable!()
    }

    fn process(&mut self) {
        // TODO: Soft error
        panic!("attempted to process UnreachableActor")
    }
}

#[derive(Protocol)]
pub enum Unreachable {}
