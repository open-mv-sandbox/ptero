use std::{
    any::Any,
    collections::{BTreeMap, VecDeque},
};

use anyhow::Error;
use tracing::{event, Level};

use crate::{ActorId, World};

/// Actor processing system trait.
pub trait System: Sized + 'static {
    type Instance;
    type Message;

    /// Perform a processing step.
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error>;
}

pub trait AnySystemEntry {
    fn insert(&mut self, actor: ActorId, slot: &mut dyn Any);

    fn remove(&mut self, actor: ActorId);

    fn enqueue(&mut self, actor: ActorId, slot: &mut dyn Any);

    fn process(&mut self, world: &mut World);
}

pub struct SystemEntry<S>
where
    S: System,
{
    system: S,
    recv: State<S>,
}

impl<S> SystemEntry<S>
where
    S: System,
{
    pub fn new(system: S) -> Self {
        Self {
            system,
            recv: State {
                instances: BTreeMap::new(),
                queue: VecDeque::new(),
            },
        }
    }
}

impl<S> AnySystemEntry for SystemEntry<S>
where
    S: System,
{
    fn insert(&mut self, actor: ActorId, slot: &mut dyn Any) {
        // TODO: Graceful error handling

        // Take the instance out
        let slot: &mut Option<S::Instance> = slot.downcast_mut().unwrap();
        let instance = slot.take().unwrap();

        self.recv.instances.insert(actor, instance);
    }

    fn remove(&mut self, actor: ActorId) {
        self.recv.instances.remove(&actor);

        // Drop pending messages of this instance
        self.recv.queue.retain(|(i, _)| *i != actor);
    }

    fn enqueue(&mut self, actor: ActorId, slot: &mut dyn Any) {
        // TODO: Graceful error handling

        // Take the message out
        let slot: &mut Option<S::Message> = slot.downcast_mut().unwrap();
        let message = slot.take().unwrap();

        self.recv.queue.push_front((actor, message));
    }

    fn process(&mut self, world: &mut World) {
        let result = self.system.process(world, &mut self.recv);

        if !self.recv.queue.is_empty() {
            event!(Level::WARN, "system did not process all pending messages");
        }

        match result {
            Ok(value) => value,
            Err(error) => {
                // TODO: What to do with this?
                event!(Level::ERROR, ?error, "system failed while processing");
            }
        }
    }
}

pub struct State<S>
where
    S: System,
{
    instances: BTreeMap<ActorId, S::Instance>,
    queue: VecDeque<(ActorId, S::Message)>,
}

impl<S> State<S>
where
    S: System,
{
    pub fn next(&mut self) -> Option<(ActorId, &mut S::Instance, S::Message)> {
        loop {
            let (actor, message) = if let Some(value) = self.queue.pop_front() {
                value
            } else {
                return None;
            };

            if !self.instances.contains_key(&actor) {
                event!(Level::ERROR, "failed to find instance for message");
                continue;
            }

            let instance = self.instances.get_mut(&actor).unwrap();
            return Some((actor, instance, message));
        }
    }

    pub fn get(&self, actor: ActorId) -> Option<&S::Instance> {
        self.instances.get(&actor)
    }

    pub fn get_mut(&mut self, actor: ActorId) -> Option<&mut S::Instance> {
        self.instances.get_mut(&actor)
    }
}
