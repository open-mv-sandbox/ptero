use std::{any::Any, sync::Arc};

use crossbeam_queue::SegQueue;
use stewart_local::{Dispatcher, Factory};

use crate::actors::Actors;

/// Shared cross-thread actor world.
///
/// Central store for running actors and messages, that executors use to get things to do.
pub struct World {
    queue: SegQueue<WorldTask>,
    actors: Arc<Actors>,
}

impl World {
    pub fn new() -> Arc<Self> {
        let queue = SegQueue::new();
        let actors = Arc::new(Actors::new());

        Arc::new(Self { queue, actors })
    }
}

impl World {
    pub(crate) fn next_task(&self) -> Option<WorldTask> {
        self.queue.pop()
    }

    pub(crate) fn actors(&self) -> &Actors {
        &self.actors
    }

    pub fn start(&self, factory: Box<dyn Factory>) {
        let task = WorldTask::Start { factory };
        self.queue.push(task);
    }
}

impl Dispatcher for World {
    fn send(&self, _actor_id: usize, address: usize, message: Box<dyn Any>) {
        // TODO: Consider downcasting at this point to bin messages in contiguous queues,
        // maybe even avoiding the need for Box altogether by granting a memory slot in-line.

        let task = WorldTask::Message { address, message };
        self.queue.push(task);
    }

    fn start(&self, _actor_id: usize, factory: Box<dyn Factory>) {
        // TODO: It makes sense to have this more executor-local. If we queue up with the local
        // executor first, we can keep the memory thread core-local. This is better for
        // performance. Then if an executor is out of work to do it can 'steal' an actor from
        // another executor.
        // TODO: Track hierarchy.

        self.start(factory);
    }
}

/// TODO: This is a naive FIFO task queue for now, but later we can more efficiently handle
/// messages relevant to one actor in series for cache-local efficiency.
pub enum WorldTask {
    Message {
        address: usize,
        message: Box<dyn Any>,
    },
    Start {
        factory: Box<dyn Factory>,
    },
}
