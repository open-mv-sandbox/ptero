use std::any::Any;

use anyhow::{Context, Error};
use tracing::{event, Level};

use crate::{Actor, After, Id, Messages, System};

pub trait AnyActorSlot {
    fn enqueue(&mut self, slot: &mut dyn Any) -> Result<(), Error>;

    fn process(&mut self, system: &mut System, id: Id) -> After;
}

pub struct ActorSlot<A>
where
    A: Actor,
{
    data: Messages<A::Message>,
    actor: A,
}

impl<A> ActorSlot<A>
where
    A: Actor,
{
    pub fn new(actor: A) -> Self {
        Self {
            data: Messages::new(),
            actor,
        }
    }
}

impl<A> AnyActorSlot for ActorSlot<A>
where
    A: Actor,
{
    fn enqueue(&mut self, slot: &mut dyn Any) -> Result<(), Error> {
        let slot: &mut Option<A::Message> =
            slot.downcast_mut().context("failed to downcast bin")?;
        let message = slot.take().context("slot was empty")?;
        self.data.enqueue(message);

        Ok(())
    }

    fn process(&mut self, system: &mut System, id: Id) -> After {
        let result = self.actor.process(system, id, &mut self.data);
        let after = handle_process_result(result);

        if self.data.has_queued() {
            event!(Level::WARN, "actor did not process all pending messages");
        }

        if after == After::Stop {
            return After::Stop;
        }

        After::Continue
    }
}

fn handle_process_result(result: Result<After, Error>) -> After {
    match result {
        Ok(value) => value,
        Err(error) => {
            // TODO: What to do with this?
            event!(Level::ERROR, ?error, "actor failed while processing");

            After::Continue
        }
    }
}
