//! Helper types for storing and calling actors dynamically.

use anyhow::{Context, Error};
use family::any::AnyOption;
use tracing::{event, Level};

use crate::{Actor, After, System};

pub trait AnyActor {
    fn reduce(&mut self, system: &mut System, message: &mut dyn AnyOption) -> Result<After, Error>;

    fn process(&mut self, system: &mut System) -> Result<After, Error>;
}

impl<A> AnyActor for A
where
    A: Actor,
{
    fn reduce(&mut self, system: &mut System, message: &mut dyn AnyOption) -> Result<After, Error> {
        let message = match message.downcast::<A::Family>() {
            Some(message) => message,
            None => {
                // This is not an error with the actor, but with the sending actor
                // TODO: Pass errors back
                event!(Level::ERROR, "incorrect dynamic message type");
                return Ok(After::Nothing);
            }
        };

        let message = message.take().context("message was already taken")?;
        Actor::reduce(self, system, message.0)
    }

    fn process(&mut self, system: &mut System) -> Result<After, Error> {
        Actor::process(self, system)
    }
}
