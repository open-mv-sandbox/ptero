use std::any::Any;

use anyhow::{Context, Error};
use tracing::{event, Level};

use crate::{Actor, After, Id, System};

pub trait AnyActorSlot {
    /// Handle the message in the slot, returns true if needing to be queued, false if processed
    /// immediately.
    fn handle(&mut self, slot: &mut dyn Any) -> Result<(), Error>;

    /// Handle processing if queued.
    fn process(&mut self, system: &mut System, id: Id) -> After;
}

pub struct ActorSlot<A>
where
    A: Actor,
{
    pub bin: Vec<A::Message>,
    pub actor: A,
}

impl<A> AnyActorSlot for ActorSlot<A>
where
    A: Actor,
{
    fn handle(&mut self, slot: &mut dyn Any) -> Result<(), Error> {
        let slot: &mut Option<A::Message> =
            slot.downcast_mut().context("failed to downcast bin")?;
        let message = slot.take().context("slot was empty")?;
        self.bin.push(message);

        Ok(())
    }

    fn process(&mut self, system: &mut System, id: Id) -> After {
        for message in self.bin.drain(..) {
            let result = self.actor.handle(system, id, message);
            let after = handle_process_result(result);

            if after == After::Stop {
                return After::Stop;
            }
        }

        After::Continue
    }
}

fn handle_process_result(result: Result<After, Error>) -> After {
    match result {
        Ok(value) => value,
        Err(error) => {
            // TODO: What to do with this?
            event!(
                Level::ERROR,
                ?error,
                "actor failed while processing message"
            );

            After::Continue
        }
    }
}
