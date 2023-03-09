use std::any::Any;

use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{After, Id, Info};

/// Thread-local actor collection and lifetime manager.
pub struct System {
    actors: Arena<ActorEntry>,
    pending_start: Vec<Index>,
    root: Index,
}

impl System {
    /// Create a new empty `System`.
    pub fn new() -> Self {
        let mut actors = Arena::new();

        // Insert a no-op root actor for tracking purposes
        let actor = ActorEntry {
            debug_name: "Root",
            span: Span::current(),
            actor: None,
        };
        let root = actors.insert(actor);

        Self {
            actors,
            pending_start: Vec::new(),
            root,
        }
    }

    /// Get root actor ID.
    pub fn root_id(&self) -> Id {
        Id { index: self.root }
    }

    /// Create an actor on the system.
    ///
    /// The actor's address will not be available for handling messages until `start` is called.
    pub fn create_actor<A: 'static>(&mut self, parent: Id) -> Result<Info<A>, CreateActorError> {
        // Continual span is inherited from the create addr callsite
        let span = Span::current();

        // Link to the parent
        self.actors
            .get_mut(parent.index)
            .ok_or(CreateActorError::ParentDoesNotExist)?;

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
        A: 'static,
    {
        event!(Level::INFO, "starting actor");

        // Remove pending, starting is what it's pending for
        let index = self
            .pending_start
            .iter()
            .position(|i| *i == info.index)
            .ok_or(StartActorError::ActorNotPending)?;
        self.pending_start.remove(index);

        // Retrieve the slot
        let entry = self
            .actors
            .get_mut(info.index)
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

    /// Temporarily borrow an actor without taking it.
    pub fn get_mut<A>(&mut self, id: Id) -> Option<&mut A>
    where
        A: 'static,
    {
        let index = id.index;

        let entry = self.actors.get_mut(index)?;
        let actor = entry.actor.as_mut()?;

        actor.as_mut().downcast_mut()
    }

    pub fn borrow_actor<A>(&mut self, id: Id) -> Result<(Span, Box<A>), BorrowError>
    where
        A: 'static,
    {
        // Find the actor's entry
        let entry = self
            .actors
            .get_mut(id.index)
            .ok_or(BorrowError::ActorNotFound)?;

        // Take the actor from the slot
        let actor = std::mem::replace(&mut entry.actor, None);

        // If the actor wasn't in the slot, return an error
        let actor = actor.ok_or(BorrowError::ActorNotAvailable {
            name: entry.debug_name,
        })?;

        // Downcast the actor to the desired type
        // TODO: Return the actor again on failure, or prevent it from being taken in the
        // first place
        let actor = actor
            .downcast()
            .map_err(|_| BorrowError::ActorInvalidType)?;

        Ok((entry.span.clone(), actor))
    }

    pub fn return_actor<A>(
        &mut self,
        id: Id,
        actor: Box<A>,
        after: After,
    ) -> Result<(), BorrowError>
    where
        A: 'static,
    {
        // TODO: Validate same type slot

        // If we got told to stop the actor, do that instead of returning
        if after == After::Stop {
            event!(Level::INFO, "stopping actor");
            drop(actor);
            self.actors.remove(id.index);
            return Ok(());
        }

        // Put the actor back in the slot
        let entry = self
            .actors
            .get_mut(id.index)
            .ok_or(BorrowError::ActorDisappeared)?;
        entry.actor = Some(actor);

        Ok(())
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
#[non_exhaustive]
pub enum CreateActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ParentDoesNotExist,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StartActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ActorNotPending,
    #[error("failed to start actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to start actor, actor at address already started")]
    ActorAlreadyStarted,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BorrowError {
    #[error("failed to borrow actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to borrow actor, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
    #[error("failed to return actor, the actor disappeared before it could be returned")]
    ActorDisappeared,
    #[error("failed to borrow or return actor, invalid type")]
    ActorInvalidType,
}
