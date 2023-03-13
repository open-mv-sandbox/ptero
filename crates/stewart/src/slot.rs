use std::any::Any;

use crate::{Actor, After, System};

use tracing::{event, Level};

pub struct ActorSlot<A>
where
    A: Actor,
{
    pub bin: Vec<A::Message>,
    pub actor: A,
}

pub trait AnyActorSlot {
    fn bin(&mut self) -> &mut dyn Any;

    fn handle_binned(&mut self, system: &mut System) -> After;
}

impl<A> AnyActorSlot for ActorSlot<A>
where
    A: Actor,
{
    fn bin(&mut self) -> &mut dyn Any {
        &mut self.bin
    }

    fn handle_binned(&mut self, system: &mut System) -> After {
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
