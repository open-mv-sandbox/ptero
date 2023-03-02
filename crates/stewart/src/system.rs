use std::collections::VecDeque;

use thunderdome::{Arena, Index};
use tracing::{event, Level};

use crate::{
    Actor, AfterReduce, AnyActor, AnyMessageSlot, Factory, Protocol, RawSystemAddr, SystemAddr,
};

/// Thread-local cooperative multitasking actor scheduler.
///
/// This executor bridges CPU threads into cooperative actor threads.
/// It does not do any scheduling in itself, this is delegated to an actor.
pub struct System {
    actors: Arena<ActorEntry>,
    queue: VecDeque<Index>,
    deferred: Vec<DeferredAction>,
}

impl System {
    pub fn new() -> Self {
        let actors = Arena::new();
        let queue = VecDeque::new();
        let deferred = Vec::new();

        Self {
            actors,
            queue,
            deferred,
        }
    }

    /// Queue starting an actor.
    pub fn start(&mut self, factory: impl Factory + 'static) {
        let action = DeferredAction::Start(Box::new(factory));
        self.deferred.push(action);
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    pub fn handle<'a, P: Protocol>(&mut self, addr: SystemAddr<P>, message: P::Message<'a>) {
        let index = addr.raw().0;

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
        let slot = AnyMessageSlot::new::<P>(&mut message_slot);
        let after = entry.actor.reduce(slot);

        // Schedule process if necessary
        match after {
            AfterReduce::Nothing => {
                // Nothing to do
            }
            AfterReduce::Process => {
                if !entry.queued {
                    self.queue.push_back(index);
                    entry.queued = true;
                }
            }
        }
    }

    pub fn run_until_idle(&mut self) {
        loop {
            self.run_deferred();

            if let Some(index) = self.queue.pop_front() {
                self.process(index)
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
        // Get an index for the actor by starting a dummy actor
        let dummy_entry = ActorEntry {
            actor: Box::new(UnreachableActor),
            queued: false,
        };
        let index = self.actors.insert(dummy_entry);

        // Start the real actor
        let addr = RawSystemAddr(index);
        let actor = factory.start(addr);

        // Replace the dummy entry
        let entry = ActorEntry {
            actor,
            queued: false,
        };
        self.actors.insert_at(index, entry);
    }

    fn process(&mut self, index: Index) -> bool {
        let entry = match self.actors.get_mut(index) {
            Some(entry) => entry,
            None => {
                event!(Level::ERROR, "invalid id in schedule");
                return true;
            }
        };

        // Perform the actor's process step
        entry.actor.process();
        entry.queued = false;

        true
    }
}

struct ActorEntry {
    actor: Box<dyn AnyActor>,
    queued: bool,
}

enum DeferredAction {
    Start(Box<dyn Factory>),
}

struct UnreachableActor;

impl Actor for UnreachableActor {
    type Protocol = Unreachable;

    fn reduce<'a>(&mut self, _message: Unreachable) -> AfterReduce {
        unreachable!()
    }

    fn process(&mut self) {
        unreachable!()
    }
}

enum Unreachable {}

impl Protocol for Unreachable {
    type Message<'a> = Self;
}
