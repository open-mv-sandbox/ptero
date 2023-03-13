use std::collections::VecDeque;

use anyhow::{Context, Error};
use thunderdome::Index;
use tracing::{event, Level};

use crate::{
    actors::Actors,
    slot::{OptionActorSlot, VecActorSlot},
    Actor, Addr, After, CreateActorError, Id, Info, StartActorError,
};

/// Thread-local actor execution system.
pub struct System {
    actors: Actors,
    queue: VecDeque<Index>,
}

impl System {
    /// Create a new empty `System`.
    pub fn new() -> Self {
        Self {
            actors: Actors::new(),
            queue: VecDeque::new(),
        }
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
    pub fn create_actor<A>(&mut self, parent: Id) -> Result<Info<A>, CreateActorError>
    where
        A: Actor,
    {
        let index = self.actors.create::<A>(parent.index)?;

        Ok(Info::new(index))
    }

    /// Start an actor on the system, making it available for handling messages.
    pub fn start_actor<A>(&mut self, info: Info<A>, actor: A) -> Result<(), StartActorError>
    where
        A: Actor,
    {
        event!(Level::INFO, "starting actor");

        let slot = VecActorSlot {
            bin: Vec::new(),
            actor,
        };
        self.actors.start_actor(info.index, Box::new(slot))?;

        Ok(())
    }

    /// Start a "mapping" actor on the system.
    ///
    /// Mapping actors are assumed to need priority processing on messages, as they only relay the
    /// message to another system.
    pub fn start_mapping_actor<A>(&mut self, info: Info<A>, actor: A) -> Result<(), StartActorError>
    where
        A: Actor,
    {
        event!(Level::INFO, "starting actor");

        let slot = OptionActorSlot { bin: None, actor };
        self.actors.start_actor(info.index, Box::new(slot))?;

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
        let entry = self.actors.get_mut(index)?;
        let slot = entry.slot.as_mut().context("actor not available")?;

        let (immediate, bin) = slot.bin_any();

        if immediate {
            let bin: &mut Option<M> = bin.downcast_mut().context("failed to downcast bin")?;
            *bin = Some(message);

            // Handle immediate
            self.process(index)?;
        } else {
            let bin: &mut Vec<M> = bin.downcast_mut().context("failed to downcast bin")?;
            bin.push(message);

            if !self.queue.contains(&index) {
                self.queue.push_back(index);
            }
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
        let entry = self.actors.get_mut(index)?;
        let span = entry.span.clone();
        let _enter = span.enter();
        let mut slot = entry.borrow_slot()?;

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
