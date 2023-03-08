use thiserror::Error;
use thunderdome::{iter::Drain, Arena, Index};
use tracing::Span;

use crate::{dynamic::AnyActor, CreateActorError};

/// Actors storage collection and ownership tracker.
#[derive(Default)]
pub struct Actors {
    arena: Arena<ActorEntry>,
}

impl Actors {
    pub fn create(
        &mut self,
        debug_name: &'static str,
        span: Span,
        parent: Option<Index>,
    ) -> Result<Index, CreateActorError> {
        // Link to the parent
        if let Some(parent) = parent {
            self.arena
                .get_mut(parent)
                .ok_or(CreateActorError::ParentDoesNotExist)?;
        }

        let entry = ActorEntry {
            debug_name,
            span,
            queued: false,
            actor: None,
        };
        let index = self.arena.insert(entry);

        Ok(index)
    }

    pub fn remove(&mut self, index: Index) -> Option<ActorEntry> {
        self.arena.remove(index)
    }

    pub fn get_mut(&mut self, index: Index) -> Option<&mut ActorEntry> {
        self.arena.get_mut(index)
    }

    pub fn borrow(
        &mut self,
        index: Index,
    ) -> Result<(&mut ActorEntry, Box<dyn AnyActor>), BorrowError> {
        // Find the actor's entry
        let entry = self
            .arena
            .get_mut(index)
            .ok_or(BorrowError::ActorNotFound)?;

        // Take the actor from the slot
        let actor = std::mem::replace(&mut entry.actor, None);

        // If the actor wasn't in the slot, return an error
        let actor = actor.ok_or(BorrowError::ActorNotAvailable {
            name: entry.debug_name,
        })?;

        Ok((entry, actor))
    }

    pub fn unborrow(
        &mut self,
        index: Index,
        actor: Box<dyn AnyActor>,
    ) -> Result<&mut ActorEntry, BorrowError> {
        let entry = self
            .arena
            .get_mut(index)
            .ok_or(BorrowError::ActorDisappeared)?;
        entry.actor = Some(actor);

        Ok(entry)
    }

    pub fn drain(&mut self) -> Drain<ActorEntry> {
        self.arena.drain()
    }
}

pub struct ActorEntry {
    /// Debugging identification name, not intended for anything other than warn/err reporting.
    pub debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    pub span: Span,
    pub queued: bool,
    pub actor: Option<Box<dyn AnyActor>>,
}

#[derive(Error, Debug)]
pub enum BorrowError {
    #[error("failed to borrow actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to borrow actor, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
    #[error("failed to return actor, the actor disappeared before it could be returned")]
    ActorDisappeared,
}
