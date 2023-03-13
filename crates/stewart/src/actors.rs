use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::slot::AnyActorSlot;

pub struct Actors {
    actors: Arena<ActorEntry>,
    pending_start: Vec<Index>,
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

        Self {
            actors,
            pending_start: Vec::new(),
            root,
        }
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

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(index);

        Ok(index)
    }

    pub fn start_actor(
        &mut self,
        index: Index,
        slot: Box<dyn AnyActorSlot>,
    ) -> Result<(), StartActorError> {
        // Remove pending, starting is what it's pending for
        let pending_index = self
            .pending_start
            .iter()
            .position(|i| *i == index)
            .ok_or(StartActorError::ActorNotPending)?;
        self.pending_start.remove(pending_index);

        self.get_mut(index)?.return_slot(slot)?;

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
        let entry = self.remove(index)?;

        let _enter = entry.span.enter();
        event!(Level::INFO, "actor failed to start in time");

        Ok(())
    }

    pub fn get_mut(&mut self, index: Index) -> Result<&mut ActorEntry, BorrowError> {
        // Find the actor's entry
        let entry = self
            .actors
            .get_mut(index)
            .ok_or(BorrowError::ActorNotFound)?;

        Ok(entry)
    }

    pub fn remove(&mut self, index: Index) -> Result<ActorEntry, Error> {
        self.actors.remove(index).context("actor doesn't exist")
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

pub struct ActorEntry {
    /// Debugging identification name, not intended for anything other than warn/err reporting.
    pub debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    pub span: Span,
    pub slot: Option<Box<dyn AnyActorSlot>>,
}

impl ActorEntry {
    pub fn borrow_slot(&mut self) -> Result<Box<dyn AnyActorSlot>, BorrowError> {
        // Take the actor from the slot
        let slot = std::mem::replace(&mut self.slot, None);

        // If the actor wasn't in the slot, return an error
        let slot = slot.ok_or(BorrowError::ActorNotAvailable {
            name: self.debug_name,
        })?;

        Ok(slot)
    }

    pub fn return_slot(&mut self, slot: Box<dyn AnyActorSlot>) -> Result<(), BorrowError> {
        // Put the actor back in the slot
        // TODO: Check if already present
        self.slot = Some(slot);

        Ok(())
    }
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
    #[error("failed to start actor, error while returning to slot")]
    BorrowError(#[from] BorrowError),
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BorrowError {
    #[error("failed to borrow actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to borrow actor, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
}
