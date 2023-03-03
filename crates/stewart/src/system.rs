use std::collections::VecDeque;

use thunderdome::{Arena, Index};
use tracing::{event, Level};

use crate::{
    utils::UnreachableActor, ActorAddr, ActorId, AfterProcess, AfterReduce, AnyActor, AnyMessage,
    Factory, Protocol,
};

// TODO: Change all unwrap/expect to soft errors

/// Thread-local cooperative multitasking actor scheduler.
///
/// This executor bridges CPU threads into cooperative actor threads.
/// It does not do any scheduling in itself, this is delegated to an actor.
pub struct System {
    actors: Arena<ActorEntry>,
    queue: VecDeque<Index>,
    deferred: Vec<DeferredAction>,
    /// Dummy placeholder, keep one around to avoid re-allocating.
    dummy_entry: Option<ActorEntry>,
}

impl System {
    pub fn new() -> Self {
        let actors = Arena::new();
        let queue = VecDeque::new();
        let deferred = Vec::new();
        let dummy_entry = ActorEntry {
            actor: Box::new(UnreachableActor),
            queued: false,
        };

        Self {
            actors,
            queue,
            deferred,
            dummy_entry: Some(dummy_entry),
        }
    }

    /// Queue starting an actor.
    pub fn start(&mut self, factory: impl Factory + 'static) {
        let action = DeferredAction::Start(Box::new(factory));
        self.deferred.push(action);
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    pub fn handle<'a, P: Protocol + 'static>(
        &mut self,
        addr: ActorAddr<P>,
        message: P::Message<'a>,
    ) {
        let index = addr.id().0;

        let entry = match self.actors.get_mut(index) {
            Some(actor) => actor,
            None => {
                // TODO: What to do with addressing error?
                event!(Level::ERROR, "failed to find actor for system address");
                return;
            }
        };

        // Let the actor reduce the message
        let mut message_slot = Some(message);
        let slot = AnyMessage::new::<P>(&mut message_slot);
        let result = entry.actor.reduce(slot);

        // Schedule process if necessary
        match result {
            Ok(AfterReduce::Nothing) => {
                // Nothing to do
            }
            Ok(AfterReduce::Process) => {
                if !entry.queued {
                    self.queue.push_back(index);
                    entry.queued = true;
                }
            }
            Err(error) => {
                // TODO: What to do with this?
                event!(Level::ERROR, "actor failed to reduce message\n{:?}", error);
            }
        }
    }

    pub fn run_until_idle(&mut self) {
        loop {
            self.run_deferred();

            if let Some(index) = self.queue.pop_front() {
                self.process_at(index);
            } else {
                return;
            };
        }
    }

    fn run_deferred(&mut self) {
        while let Some(action) = self.deferred.pop() {
            match action {
                DeferredAction::Start(factory) => self.run_deferred_start(factory),
            }
        }
    }

    fn run_deferred_start(&mut self, factory: Box<dyn Factory>) {
        event!(Level::TRACE, "starting actor");

        // Get an index for the actor by starting a dummy actor
        let dummy_entry = ActorEntry {
            actor: Box::new(UnreachableActor),
            queued: false,
        };
        let index = self.actors.insert(dummy_entry);

        // Start the real actor
        let addr = ActorId(index);
        let result = factory.start(addr);

        // Handle factory result
        match result {
            Ok(actor) => {
                let entry = ActorEntry {
                    actor,
                    queued: false,
                };
                self.actors.insert_at(index, entry);
            }
            Err(error) => {
                event!(Level::ERROR, "actor failed to start\n{:?}", error);
                self.actors.remove(index);
            }
        }
    }

    fn process_at(&mut self, index: Index) {
        if !self.actors.contains(index) {
            event!(Level::ERROR, "invalid id in schedule");
            return;
        }

        // Swap out for a dummy actor
        let dummy_entry = self.dummy_entry.take().expect("dummy entry already taken");
        let mut entry = self
            .actors
            .insert_at(index, dummy_entry)
            .expect("actor unexpectedly disappeared");

        // Perform the actor's process step
        let result = entry.actor.process(self);
        entry.queued = false;

        // Re-insert the actor
        let dummy_entry = self
            .actors
            .insert_at(index, entry)
            .expect("actor unexpectedly disappeared");
        self.dummy_entry = Some(dummy_entry);

        // Handle the result
        match result {
            Ok(AfterProcess::Nothing) => {
                // Nothing to do
            }
            Ok(AfterProcess::Stop) => {
                self.stop(index);
            }
            Err(error) => {
                event!(Level::ERROR, "actor failed to process\n{:?}", error);
            }
        }
    }

    fn stop(&mut self, index: Index) {
        event!(Level::TRACE, "stopping actor");
        self.actors.remove(index);
    }
}

struct ActorEntry {
    actor: Box<dyn AnyActor>,
    queued: bool,
}

enum DeferredAction {
    Start(Box<dyn Factory>),
}
