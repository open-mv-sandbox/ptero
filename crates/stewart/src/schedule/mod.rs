//! Actor processing scheduling.

mod schedule;

use anyhow::Error;
use family::utils::{FamilyT, MemberT};
use tracing::{event, Level};

use crate::{handler::Actor, After, Id, Info, System};

pub use self::schedule::{Schedule, ScheduleError};

pub struct QueueActor<A, M> {
    info: Info<Self>,
    schedule: Schedule,
    queue: Vec<M>,
    actor: A,
}

impl<A, M> QueueActor<A, M>
where
    A: Actor<Family = FamilyT<M>> + 'static,
    M: 'static,
{
    pub fn new(info: Info<Self>, schedule: Schedule, actor: A) -> Self {
        Self {
            info,
            schedule,
            queue: Vec::new(),
            actor,
        }
    }

    fn process(&mut self, system: &mut System) -> Result<After, Error> {
        for message in self.queue.drain(..) {
            // Call inner handle
            let message = MemberT(message);
            let result = self.actor.handle(system, message);

            let after = match result {
                Ok(value) => value,
                Err(error) => {
                    // TODO: What to do with this?
                    event!(Level::ERROR, "actor failed to handle message\n{:?}", error);
                    After::Nothing
                }
            };

            if after == After::Stop {
                return Ok(After::Stop);
            }
        }

        Ok(After::Nothing)
    }

    fn apply(system: &mut System, id: Id) -> Result<(), Error> {
        // Take the actor out of the system
        let (span, mut actor) = system.borrow_actor::<Self>(id)?;
        let _enter = span.enter();

        // Perform processing
        let result = actor.process(system);

        // Handle result
        let after = match result {
            Ok(value) => value,
            Err(error) => {
                // TODO: What to do with this?
                event!(
                    Level::ERROR,
                    "actor failed to apply queue item\n{:?}",
                    error
                );
                After::Nothing
            }
        };

        // Return the actor
        system.return_actor(id, actor, after)?;

        Ok(())
    }
}

impl<A, M> Actor for QueueActor<A, M>
where
    A: Actor<Family = FamilyT<M>> + 'static,
    M: 'static,
{
    type Family = FamilyT<M>;

    fn handle(&mut self, _system: &mut System, message: MemberT<M>) -> Result<After, Error> {
        // Queue the message
        self.queue.push(message.0);

        // Schedule for handling
        self.schedule.push(self.info.id(), Self::apply)?;

        Ok(After::Nothing)
    }
}
