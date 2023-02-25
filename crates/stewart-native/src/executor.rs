use std::{any::Any, sync::Arc};

use stewart::Next;
use stewart_local::Factory;
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
        // TODO: This is a very naive way of running an async executor, and needs a lot of
        // improvement to work with external systems, because it can't wait on external signals.

        // TODO: Execution should happen on a thread pool.
        // This executor is just a building block of such a threaded dispatching system.
        // This has some implications for handler locking that should be checked at that point.
        // For example, task scheduling should be done in a way that avoids mutex lock contention.
        // Maybe execution workers should just be given handlers to run from the scheduler, rather
        // than messages? Then there's no need for mutexes at all.

        // TODO: Message executor as actor?
        // Per-message-type actors won't work, as we very frequently want to distribute the same
        // message across multiple threads.

        while let Some(task) = self.world.next_task() {
            match task {
                WorldTask::Message { address, message } => self.execute_message(address, message),
                WorldTask::Start { factory } => self.execute_start(factory),
            }
        }
    }

    fn execute_message(&self, address: usize, message: Box<dyn Any>) {
        // Run the actor's handler
        let result = self
            .world
            .actors()
            .run(address, |actor| actor.handle_any(message));

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
        let factory = |id| factory.start(id, self.world.clone(), id);
        self.world.actors().start(factory);
    }
}
