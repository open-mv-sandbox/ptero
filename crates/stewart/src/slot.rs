use std::any::Any;

use crate::{Actor, After, System};

use tracing::{event, Level};

pub trait AnyActorSlot {
    /// Returns true if this needs the message to be handled immediately.
    fn bin_any(&mut self) -> (bool, &mut dyn Any);

    /// Handle processing if queued.
    fn process(&mut self, system: &mut System) -> After;
}

pub struct VecActorSlot<A>
where
    A: Actor,
{
    pub bin: Vec<A::Message>,
    pub actor: A,
}

impl<A> AnyActorSlot for VecActorSlot<A>
where
    A: Actor,
{
    fn bin_any(&mut self) -> (bool, &mut dyn Any) {
        (false, &mut self.bin)
    }

    fn process(&mut self, system: &mut System) -> After {
        for message in self.bin.drain(..) {
            let result = self.actor.handle(system, message);

            let after = match result {
                Ok(value) => value,
                Err(error) => {
                    // TODO: What to do with this?
                    event!(Level::ERROR, ?error, "actor failed to process message");

                    After::Nothing
                }
            };

            if after == After::Stop {
                return After::Stop;
            }
        }

        After::Nothing
    }
}

pub struct OptionActorSlot<A>
where
    A: Actor,
{
    pub bin: Option<A::Message>,
    pub actor: A,
}

impl<A> AnyActorSlot for OptionActorSlot<A>
where
    A: Actor,
{
    fn bin_any(&mut self) -> (bool, &mut dyn Any) {
        (true, &mut self.bin)
    }

    fn process(&mut self, system: &mut System) -> After {
        if let Some(message) = self.bin.take() {
            let result = self.actor.handle(system, message);

            let after = match result {
                Ok(value) => value,
                Err(error) => {
                    // TODO: What to do with this?
                    event!(Level::ERROR, ?error, "actor failed to process message");

                    After::Nothing
                }
            };

            after
        } else {
            After::Nothing
        }
    }
}
