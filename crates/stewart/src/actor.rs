use std::collections::VecDeque;

use anyhow::Error;

use crate::{Id, System};

/// Message handling interface.
///
/// TODO: Maybe this should instead be a `Process` trait, registered on an 'actor type', which can
/// process all of a *type* of message rather than just all messages for one actor.
pub trait Actor: 'static {
    type Message;

    /// Perform a processing step.
    fn process(
        &mut self,
        system: &mut System,
        id: Id,
        data: &mut ActorData<Self::Message>,
    ) -> Result<After, Error>;
}

/// The operation to perform with the actor after a message was handled.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum After {
    /// Continue running the actor.
    Continue,
    /// Stop the actor and remove it from the system.
    Stop,
}

/// TODO: If the above Actor trait rename happens, this should be named "Actor" and contain state
/// data.
pub struct ActorData<M> {
    queue: VecDeque<M>,
}

impl<M> ActorData<M> {
    pub(crate) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub(crate) fn enqueue(&mut self, message: M) {
        self.queue.push_back(message);
    }

    pub fn next(&mut self) -> Option<M> {
        self.queue.pop_front()
    }

    pub(crate) fn has_queued(&mut self) -> bool {
        !self.queue.is_empty()
    }
}
