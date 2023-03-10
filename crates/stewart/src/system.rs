use std::collections::VecDeque;

use anyhow::{Context, Error};
use thunderdome::Index;
use tracing::{event, Level};

use crate::{
    actor_tree::ActorTree, slot::ActorSlot, Actor, Addr, After, CreateActorError, Id, Info,
    Options, StartActorError,
};

/// Thread-local actor execution system.
#[derive(Default)]
pub struct System {
    actors: ActorTree,
    queue: VecDeque<Index>,
}

impl System {
    /// Create a new empty `System`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get root actor ID.
    pub fn root_id(&self) -> Id {
        Id {
            index: self.actors.root(),
        }
    }

    /// Create an actor on the system.
    ///
    /// The actor's address will not be available for handling messages until `start` is called.
    pub fn create<A>(&mut self, parent: Id) -> Result<Info<A>, CreateActorError>
    where
        A: Actor,
    {
        let index = self.actors.create::<A>(parent.index)?;
        let info = Info::new(index);

        Ok(info)
    }

    /// Start an actor on the system, making it available for handling messages.
    pub fn start<A>(
        &mut self,
        info: Info<A>,
        options: Options,
        actor: A,
    ) -> Result<(), StartActorError>
    where
        A: Actor,
    {
        event!(Level::INFO, "starting actor");

        let slot = ActorSlot {
            bin: Vec::new(),
            actor,
        };
        self.actors.start(info.index, options, Box::new(slot))?;

        Ok(())
    }

    pub fn send<M>(&mut self, addr: Addr<M>, message: impl Into<M>)
    where
        M: 'static,
    {
        let result = self.try_send(addr.index, message.into());

        // TODO: Figure out what to do with this, it may be useful to have a unified "send error"
        // system, but in some cases a definitive error may never happen until the implementor
        // decides that it's too long?
        // Some cases, it's an error with the receiver, some cases it's an error with the sender.
        // This needs more thought before making an API decision.
        if let Err(error) = result {
            event!(Level::ERROR, ?error, "failed to send message");
        }
    }

    pub fn try_send<M>(&mut self, index: Index, message: M) -> Result<(), Error>
    where
        M: 'static,
    {
        let node = self.actors.get_mut(index)?;
        let slot = node.slot.as_mut().context("actor not available")?;

        let mut message = Some(message);
        slot.handle(&mut message)?;

        if !node.options.mapping {
            // Queue for later processing
            if !self.queue.contains(&index) {
                self.queue.push_back(index);
            }
        } else {
            // Process in-place
            self.process(index)?;
        }

        Ok(())
    }

    pub fn run_until_idle(&mut self) -> Result<(), Error> {
        self.actors.cleanup_pending()?;

        while let Some(index) = self.queue.pop_front() {
            self.process(index)?;

            self.actors.cleanup_pending()?;
        }

        Ok(())
    }

    fn process(&mut self, index: Index) -> Result<(), Error> {
        // Borrow the actor
        let node = self.actors.get_mut(index)?;
        let span = node.span.clone();
        let _enter = span.enter();
        let mut slot = node.borrow_slot()?;

        // Run the actor's handler
        let after = slot.process(self);

        // If we got told to stop the actor, do that instead of returning
        if after == After::Stop {
            event!(Level::INFO, "stopping actor");
            drop(slot);
            self.actors.remove(index)?;
            return Ok(());
        }

        // Return the actor
        self.actors.get_mut(index)?.return_slot(slot)?;

        Ok(())
    }
}

impl Drop for System {
    fn drop(&mut self) {
        let debug_names = self.actors.debug_names();

        if !debug_names.is_empty() {
            event!(
                Level::WARN,
                ?debug_names,
                "actors not stopped before system drop"
            );
        }
    }
}
