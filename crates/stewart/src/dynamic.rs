//! Helper types for storing and calling actors dynamically.

use std::any::Any;

use anyhow::{Context, Error};
use family::any::AnyOption;
use tracing::{event, Level};

use crate::{Actor, After, System};

// TODO: Replace family-downcast with Handler<F>-downcast instead, and then .as_handler() on
// AnyActor.

pub trait AnyActor {
    fn as_any(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn handle(&mut self, system: &mut System, message: &mut dyn AnyOption) -> Result<After, Error>;
}

impl<A> AnyActor for A
where
    A: Actor + 'static,
{
    fn as_any(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn handle(&mut self, system: &mut System, message: &mut dyn AnyOption) -> Result<After, Error> {
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
        Actor::handle(self, system, message.0)
    }
}
