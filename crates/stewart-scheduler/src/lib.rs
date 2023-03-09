mod process;

use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use anyhow::Error;
use stewart::{handler::After, Id, Info, System};
use thiserror::Error;
use tracing::{event, Level};

pub use self::process::Process;

/// Shared thread-local processing schedule.
#[derive(Clone, Default)]
pub struct Schedule {
    queue: Rc<RefCell<VecDeque<Item>>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an actor to the schedule for processing.
    pub fn push<A>(&mut self, info: Info<A>) -> Result<(), ScheduleError>
    where
        A: Process + 'static,
    {
        let mut queue = self.queue.try_borrow_mut().map_err(|_| ScheduleError)?;

        // If the queue already contains this actor, skip
        if queue.iter().any(|v| v.id == info.id()) {
            return Ok(());
        }

        // Add the actor to the queue
        let item = Item {
            id: info.id(),
            apply: apply_process::<A>,
        };
        queue.push_back(item);

        Ok(())
    }

    /// Run all queued actor processing tasks, until none remain.
    ///
    /// Running a process task may spawn new process tasks, so this is not guaranteed to ever
    /// return.
    pub fn run_until_idle(&self, system: &mut System) -> Result<(), Error> {
        system.cleanup_pending()?;

        while let Some(item) = self.take_next()? {
            // Apply process
            (item.apply)(system, item.id)?;

            system.cleanup_pending()?;
        }

        Ok(())
    }

    fn take_next(&self) -> Result<Option<Item>, ScheduleError> {
        let item = self
            .queue
            .try_borrow_mut()
            .map_err(|_| ScheduleError)?
            .pop_front();
        Ok(item)
    }
}

#[derive(Error, Debug)]
#[error("schedule is unavailable")]
pub struct ScheduleError;

struct Item {
    id: Id,
    apply: fn(&mut System, Id) -> Result<(), Error>,
}

fn apply_process<A: Process + 'static>(system: &mut System, id: Id) -> Result<(), Error> {
    // Take the actor out of the system
    let (span, mut actor) = system.borrow_actor::<A>(id)?;
    let _enter = span.enter();

    // Perform processing
    let result = actor.process(system);

    // Handle the result
    let after = match result {
        Ok(value) => value,
        Err(error) => {
            // TODO: What to do with this?
            event!(Level::ERROR, "actor failed to process\n{:?}", error);
            After::Nothing
        }
    };

    // Return the actor
    system.return_actor(id, actor, after == After::Stop)?;

    Ok(())
}
