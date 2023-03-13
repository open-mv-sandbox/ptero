use std::collections::VecDeque;

use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::Index;
use tracing::{event, Level};

use crate::{
    actors::{Actors, BorrowError},
    slot::ActorSlot,
    Actor, Addr, After, CreateActorError, Id, Info,
};

/// Thread-local actor execution system.
pub struct System {
    actors: Actors,
    pending_start: Vec<Index>,
    queue: VecDeque<Index>,
    // TODO: Message mapping/apply addresses.
}

impl System {
    /// Create a new empty `System`.
    pub fn new() -> Self {
        Self {
            actors: Actors::new(),
            pending_start: Vec::new(),
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

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(index);

        Ok(Info::new(index))
    }

    /// Start an actor on the system, making it available for handling messages.
    pub fn start_actor<A>(&mut self, info: Info<A>, actor: A) -> Result<(), StartActorError>
    where
        A: Actor,
    {
        event!(Level::INFO, "starting actor");

        // Remove pending, starting is what it's pending for
        let index = self
            .pending_start
            .iter()
            .position(|i| *i == info.index)
            .ok_or(StartActorError::ActorNotPending)?;
        self.pending_start.remove(index);

        // Fill the actor slot
        let slot = ActorSlot {
            bin: Vec::new(),
            actor,
        };
        self.actors
            .return_actor(info.index, Box::new(slot), After::Nothing)?;

        Ok(())
    }

    pub fn send<M>(&mut self, addr: Addr<M>, message: impl Into<M>)
    where
        M: 'static,
    {
        let result = self.try_send(addr, message.into());

        // TODO: Figure out what to do with this, it may be useful to have a unified "send error"
        // system, but in some cases a definitive error may never happen until the implementor
        // decides that it's too long?
        // Some cases, it's an error with the receiver, some cases it's an error with the sender.
        // This needs more thought before making an API decision.
        if let Err(error) = result {
            event!(Level::ERROR, ?error, "failed to send message");
        }
    }

    pub fn try_send<M>(&mut self, addr: Addr<M>, message: M) -> Result<(), Error>
    where
        M: 'static,
    {
        let slot = self.actors.get_mut(addr.index)?;

        let bin: &mut Vec<M> = slot
            .bin()
            .downcast_mut()
            .context("failed to downcast bin")?;
        bin.push(message);

        if !self.queue.contains(&addr.index) {
            self.queue.push_back(addr.index);
        }

        Ok(())
    }

    pub fn run_until_idle(&mut self) -> Result<(), Error> {
        self.cleanup_pending()?;

        while let Some(index) = self.queue.pop_front() {
            let (span, mut actor) = self.actors.borrow_actor(index)?;
            let _enter = span.enter();

            // Run the actor's handler
            let after = actor.handle_binned(self);

            self.actors.return_actor(index, actor, after)?;
        }

        Ok(())
    }

    /// Clean up actors that didn't start in time, and thus failed.
    fn cleanup_pending(&mut self) -> Result<(), Error> {
        // Intentionally in reverse order, clean up children before parents
        while let Some(index) = self.pending_start.pop() {
            self.cleanup_pending_at(index)?;
        }

        Ok(())
    }

    fn cleanup_pending_at(&mut self, index: Index) -> Result<(), Error> {
        self.actors.fail_remove(index, "failed to start in time")?;
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

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StartActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ActorNotPending,
    #[error("failed to start actor, error while returning to slot")]
    BorrowError(#[from] BorrowError),
}
