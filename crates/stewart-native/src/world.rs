use std::{any::Any, sync::Arc};

use crossbeam_queue::SegQueue;
use stewart::Factory;

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

    pub(crate) fn send(&self, address: usize, message: Box<dyn Any>) {
        let task = WorldTask::Message { address, message };
        self.queue.push(task);
    }

    pub fn start(&self, factory: Box<dyn Factory>) {
        let task = WorldTask::Start { factory };
        self.queue.push(task);
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
