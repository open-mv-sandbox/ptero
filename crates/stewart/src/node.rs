use thiserror::Error;
use tracing::Span;

use crate::slot::AnyActorSlot;

pub struct Node {
    /// Debugging identification name, not intended for anything other than warn/err reporting.
    pub debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    pub span: Span,
    pub options: Options,
    pub slot: Option<Box<dyn AnyActorSlot>>,
}

impl Node {
    pub fn new(debug_name: &'static str, span: Span) -> Self {
        Self {
            debug_name,
            span,
            options: Options::default(),
            slot: None,
        }
    }

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

#[derive(Default)]
pub struct Options {
    pub mapping: bool,
}

impl Options {
    pub fn mapping(mut self) -> Self {
        self.mapping = true;
        self
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum BorrowError {
    #[error("failed to borrow actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to borrow actor, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
}
