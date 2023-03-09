use std::any::Any;

use anyhow::{Context, Error};
use family::Family;
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{Actor, After, Id, Info, Sender};

/// Thread-local cooperative multitasking actor scheduler.
#[derive(Default)]
pub struct System {
    actors: Arena<ActorEntry>,
    pending_start: Vec<Index>,
}

impl System {
    /// Create a new thread-local system with no actors.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an actor on the system.
    ///
    /// The actor's address will not be available for handling messages until `start` is called.
    pub fn create_actor<A: Actor + 'static>(
        &mut self,
        parent: Id,
    ) -> Result<Info<A>, CreateActorError> {
        // Continual span is inherited from the create addr callsite
        let span = Span::current();

        // Link to the parent
        if let Some(parent) = parent.0 {
            self.actors
                .get_mut(parent)
                .ok_or(CreateActorError::ParentDoesNotExist)?;
        }

        // Create the entry
        let entry = ActorEntry {
            debug_name: debug_name::<A>(),
            span,
            actor: None,
        };
        let index = self.actors.insert(entry);

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(index);

        Ok(Info::new(index))
    }

    /// Start an actor on the system, making it available for handling messages.
    pub fn start_actor<A>(&mut self, info: Info<A>, actor: A) -> Result<(), StartActorError>
    where
        A: Actor + 'static,
    {
        event!(Level::INFO, "starting actor");

        // Remove pending, starting is what it's pending for
        let index = self
            .pending_start
            .iter()
            .position(|i| *i == info.index())
            .ok_or(StartActorError::ActorNotPending)?;
        self.pending_start.remove(index);

        // Retrieve the slot
        let entry = self
            .actors
            .get_mut(info.index())
            .ok_or(StartActorError::ActorNotFound)?;

        // Fill the slot
        let actor = Box::new(actor);
        entry.actor = Some(actor);

        Ok(())
    }

    /// Clean up actors that didn't start in time, and thus failed.
    pub fn cleanup_pending(&mut self) -> Result<(), Error> {
        // Intentionally in reverse order, clean up children before parents
        while let Some(index) = self.pending_start.pop() {
            self.cleanup_pending_at(index)?;
        }

        Ok(())
    }

    fn cleanup_pending_at(&mut self, index: Index) -> Result<(), Error> {
        let entry = self
            .actors
            .remove(index)
            .context("pending actor address doesn't exist")?;

        let _enter = entry.span.enter();
        event!(Level::INFO, "failed to start in time, cleaning up");

        Ok(())
    }

    pub fn borrow_actor<A: 'static>(&mut self, id: Id) -> Result<(Span, Box<A>), BorrowError> {
        // TODO: Cleanup borrow/return now that only public ones are called

        let index = id.0.ok_or(BorrowError::CantBorrowRoot)?;
        let (span, actor) = self.borrow_actor_inner(index)?;

        // Downcast the actor to the desired type
        // TODO: Return the actor again on failure, or prevent it from being taken in the
        // first place
        let actor = actor.downcast().map_err(|_| BorrowError::ActorWrongType)?;

        Ok((span, actor))
    }

    fn borrow_actor_inner(&mut self, index: Index) -> Result<(Span, Box<dyn Any>), BorrowError> {
        // Find the actor's entry
        let entry = self
            .actors
            .get_mut(index)
            .ok_or(BorrowError::ActorNotFound)?;

        // Take the actor from the slot
        let actor = std::mem::replace(&mut entry.actor, None);

        // If the actor wasn't in the slot, return an error
        let actor = actor.ok_or(BorrowError::ActorNotAvailable {
            name: entry.debug_name,
        })?;

        Ok((entry.span.clone(), actor))
    }

    pub fn return_actor<A: Actor + 'static>(
        &mut self,
        id: Id,
        actor: Box<A>,
        after: After,
    ) -> Result<(), BorrowError> {
        // TODO: Validate same type
        let index = id.0.ok_or(BorrowError::CantBorrowRoot)?;

        // If we got told to stop the actor, do that instead of returning
        if after == After::Stop {
            event!(Level::INFO, "stopping actor");
            drop(actor);
            self.actors.remove(index);
            return Ok(());
        }

        // Put the actor back in the slot
        let entry = self
            .actors
            .get_mut(index)
            .ok_or(BorrowError::ActorDisappeared)?;
        entry.actor = Some(actor);

        Ok(())
    }

    // TODO: Clean this up
    pub fn get_mut<F: Family>(&mut self, addr: Sender<F>) -> &mut dyn Any {
        let entry = self.actors.get_mut(addr.index).unwrap();
        let actor = entry.actor.as_mut().unwrap();
        actor.as_mut()
    }
}

impl Drop for System {
    fn drop(&mut self) {
        let mut debug_names = Vec::new();
        for (_, entry) in self.actors.drain() {
            debug_names.push(entry.debug_name);
        }

        if !debug_names.is_empty() {
            let debug_names = debug_names.join(",");
            event!(
                Level::WARN,
                debug_names,
                "actors not stopped before system drop"
            );
        }
    }
}

fn debug_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    let before_generics = name.split('<').next().unwrap_or("Unknown");
    let after_modules = before_generics.split("::").last().unwrap_or("Unknown");
    after_modules
}

struct ActorEntry {
    /// Debugging identification name, not intended for anything other than warn/err reporting.
    debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    span: Span,
    actor: Option<Box<dyn Any>>,
}

#[derive(Error, Debug)]
pub enum CreateActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ParentDoesNotExist,
}

#[derive(Error, Debug)]
pub enum StartActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ActorNotPending,
    #[error("failed to start actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to start actor, actor at address already started")]
    ActorAlreadyStarted,
}

#[derive(Error, Debug)]
pub enum BorrowError {
    #[error("failed to borrow actor, cannot borrow root actor")]
    CantBorrowRoot,
    #[error("failed to borrow actor, wrong type")]
    ActorWrongType,
    #[error("failed to borrow actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to borrow actor, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
    #[error("failed to return actor, the actor disappeared before it could be returned")]
    ActorDisappeared,
}
