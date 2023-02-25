use std::{any::Any, sync::Arc};

use stewart::Next;
use stewart::{Context, Factory};
use tracing::{event, Level};

use crate::{world::WorldTask, World};

/// Thread-local actor executor.
pub struct ThreadExecutor {
    world: Arc<World>,
}

impl ThreadExecutor {
    pub fn new(world: Arc<World>) -> Self {
        Self { world }
    }

    /// Run the executor until there's no more work to do.
    pub fn run_until_idle(&self) {
        while let Some(task) = self.world.next_task() {
            match task {
                WorldTask::Message { address, message } => self.execute_message(address, message),
                WorldTask::Start { factory } => self.execute_start(factory),
            }
        }
    }

    fn execute_message(&self, address: usize, message: Box<dyn Any>) {
        // Run the actor's handler
        let ctx = ThreadExecutorContext { this: self };
        let result = self
            .world
            .actors()
            .run(address, |actor| actor.handle_any(&ctx, message));

        // TODO: What should we do with the error?
        let next = match result {
            Ok(Ok(next)) => next,
            Err(error) => {
                event!(Level::ERROR, "error while finding actor\n{:?}", error);
                return;
            }
            Ok(Err(error)) => {
                event!(
                    Level::ERROR,
                    "error while running actor.handle\n{:?}",
                    error
                );
                return;
            }
        };

        // If the actor wants to remove itself, remove it
        if next == Next::Stop {
            self.world.actors().stop(address);
        }
    }

    fn execute_start(&self, factory: Box<dyn Factory>) {
        // TODO: Track hierarchy
        let ctx = ThreadExecutorContext { this: self };
        let factory = |id| factory.start(&ctx, id);
        self.world.actors().start(factory);
    }
}

struct ThreadExecutorContext<'a> {
    this: &'a ThreadExecutor,
}

impl<'a> Context for ThreadExecutorContext<'a> {
    fn send_any(&self, address: usize, message: Box<dyn Any>) {
        // TODO: Consider downcasting at this point to bin messages in contiguous queues,
        // maybe even avoiding the need for Box altogether by granting a memory slot in-line.

        self.this.world.send(address, message);
    }

    fn start_any(&self, factory: Box<dyn Factory>) {
        // TODO: Reorganize the pattern in which actors are stored and run.
        // Actors should be associated with an executor, and the executor should handle its own
        // actors first. When an executor no longer has local actors to handle messages for, it
        // should 'steal' actors from other executors to distribute work.

        // TODO: Track hierarchy.

        self.this.world.start(factory);
    }
}
