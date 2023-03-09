use std::collections::VecDeque;

use anyhow::Error;
use stewart::{Actor, ActorT, After, Id, SenderT, System};
use tracing::{event, instrument, Level};

pub trait Process {
    fn process(&mut self, system: &mut System) -> Result<After, Error>;
}

pub struct ProcessItem {
    id: Id,
    apply: fn(&mut System, Id) -> Result<(), Error>,
}

impl ProcessItem {
    pub fn new<A: Process + Actor + 'static>(id: Id) -> Self {
        Self {
            id,
            apply: apply_process::<A>,
        }
    }
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
    system.return_actor(id, actor, after)?;

    Ok(())
}

#[instrument("scheduler", skip_all)]
pub fn start_scheduler(
    system: &mut System,
    parent: Option<Id>,
) -> Result<(SenderT<ProcessItem>, Id), Error> {
    // TODO: Encapsulate Id

    // Create the scheduler
    let info = system.create_actor(parent)?;
    let actor = SchedulerActor {
        queue: VecDeque::new(),
    };
    system.start_actor(info, actor)?;

    Ok((info.sender(), info.id()))
}

struct SchedulerActor {
    queue: VecDeque<ProcessItem>,
}

impl ActorT for SchedulerActor {
    type Message = ProcessItem;

    fn handle(&mut self, _system: &mut System, message: ProcessItem) -> Result<After, Error> {
        // TODO: Avoid duplicates
        self.queue.push_back(message);
        Ok(After::Nothing)
    }
}

/// Run all queued actor processing tasks, until none remain.
///
/// Running a process task may spawn new process tasks, so this is not guaranteed to ever
/// return.
pub fn run_until_idle(system: &mut System, scheduler: Id) -> Result<(), Error> {
    system.cleanup_pending()?;

    while let Some(item) = take_next(system, scheduler) {
        // Apply process
        (item.apply)(system, item.id)?;

        system.cleanup_pending()?;
    }

    Ok(())
}

fn take_next(system: &mut System, scheduler: Id) -> Option<ProcessItem> {
    let scheduler = system
        .get_mut::<SchedulerActor>(scheduler)
        .expect("failed to get scheduler");
    scheduler.queue.pop_front()
}
