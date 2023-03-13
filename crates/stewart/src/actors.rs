use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{slot::AnyActorSlot, After};

pub struct Actors {
    actors: Arena<ActorEntry>,
    root: Index,
}

impl Actors {
    pub fn new() -> Self {
        let mut actors = Arena::new();

        // Insert a no-op root actor for tracking purposes
        let actor = ActorEntry {
            debug_name: "Root",
            span: Span::current(),
            slot: None,
        };
        let root = actors.insert(actor);

        Self { actors, root }
    }

    pub fn root(&self) -> Index {
        self.root
    }

    pub fn create<A>(&mut self, parent: Index) -> Result<Index, CreateActorError> {
        // Continual span is inherited from the create addr callsite
        let span = Span::current();

        // Link to the parent
        self.actors
            .get_mut(parent)
            .ok_or(CreateActorError::ParentDoesNotExist)?;

        // Create the entry
        let entry = ActorEntry {
            debug_name: debug_name::<A>(),
            span,
            slot: None,
        };
        let index = self.actors.insert(entry);

        Ok(index)
    }

    pub fn fail_remove(&mut self, index: Index, reason: &str) -> Result<(), Error> {
        let entry = self
            .actors
            .remove(index)
            .context("failed to remove actor for failure, doesn't exist")?;

        let _enter = entry.span.enter();
        event!(Level::INFO, reason, "actor failed, removing");

        Ok(())
    }

    pub fn get_mut(&mut self, index: Index) -> Result<&mut dyn AnyActorSlot, BorrowError> {
        let entry = self
            .actors
            .get_mut(index)
            .ok_or(BorrowError::ActorNotFound)?;
        let slot = entry.slot.as_mut().ok_or(BorrowError::ActorNotAvailable {
            name: entry.debug_name,
        })?;

        Ok(slot.as_mut())
    }

    pub fn borrow_actor(
        &mut self,
        index: Index,
    ) -> Result<(Span, Box<dyn AnyActorSlot>), BorrowError> {
        // Find the actor's entry
        let entry = self
            .actors
            .get_mut(index)
            .ok_or(BorrowError::ActorNotFound)?;

        // Take the actor from the slot
        let slot = std::mem::replace(&mut entry.slot, None);

        // If the actor wasn't in the slot, return an error
        let slot = slot.ok_or(BorrowError::ActorNotAvailable {
            name: entry.debug_name,
        })?;

        Ok((entry.span.clone(), slot))
    }

    pub fn return_actor(
        &mut self,
        index: Index,
        slot: Box<dyn AnyActorSlot>,
        after: After,
    ) -> Result<(), BorrowError> {
        // If we got told to stop the actor, do that instead of returning
        if after == After::Stop {
            event!(Level::INFO, "stopping actor");
            drop(slot);
            self.actors.remove(index);
            return Ok(());
        }

        // Put the actor back in the slot
        // TODO: Check if already present
        let entry = self
            .actors
            .get_mut(index)
            .ok_or(BorrowError::ActorDisappeared)?;
        entry.slot = Some(slot);

        Ok(())
    }

    /// Get the debug names of all active actors, except root.
    pub fn debug_names(&self) -> Vec<&'static str> {
        let mut debug_names = Vec::new();

        for (id, entry) in &self.actors {
            if id == self.root {
                continue;
            }

            debug_names.push(entry.debug_name);
        }

        debug_names
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
    slot: Option<Box<dyn AnyActorSlot>>,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CreateActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ParentDoesNotExist,
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
}
