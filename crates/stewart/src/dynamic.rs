//! Helper types for storing and calling actors dynamically.

use anyhow::Error;
use family::any::AnyFamilyMember;
use tracing::{event, Level};

use crate::{Actor, AfterProcess, AfterReduce, System};

pub trait AnyActor {
    fn reduce(&mut self, message: Box<dyn AnyFamilyMember + '_>) -> Result<AfterReduce, Error>;

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

impl<A> AnyActor for A
where
    A: Actor,
{
    fn reduce(&mut self, message: Box<dyn AnyFamilyMember + '_>) -> Result<AfterReduce, Error> {
        let message = match message.downcast::<A::Family>() {
            Some(message) => message,
            None => {
                // This is not an error with the actor, but with the sending actor
                // TODO: Pass errors back
                event!(Level::ERROR, "incorrect dynamic message type");
                return Ok(AfterReduce::Nothing);
            }
        };

        Actor::reduce(self, message.0)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        Actor::process(self, system)
    }
}
