use thiserror::Error;
use tracing::Span;

use crate::slot::AnyActorSlot;

pub struct Node {
    /// Debugging identification name, not intended for anything other than warn/err reporting.
    debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    span: Span,
    options: Options,
    slot: Option<Box<dyn AnyActorSlot>>,
}

impl Node {
    pub fn new(span: Span) -> Self {
        Self {
            debug_name: "PendingStart",
            span,
            options: Options::default(),
            slot: None,
        }
    }

    pub fn debug_name(&self) -> &'static str {
        self.debug_name
    }

    pub fn set_debug_name(&mut self, value: &'static str) {
        self.debug_name = value;
    }

    pub fn span(&self) -> Span {
        self.span.clone()
    }

    pub fn options(&self) -> &Options {
        &self.options
    }

    pub fn set_options(&mut self, options: Options) {
        self.options = options;
    }

    pub fn slot_mut(&mut self) -> Option<&mut Box<dyn AnyActorSlot>> {
        self.slot.as_mut()
    }

    pub fn take(&mut self) -> Result<Box<dyn AnyActorSlot>, TakeError> {
        // Take the actor from the slot
        let slot = std::mem::replace(&mut self.slot, None);

        // If the actor wasn't in the slot, return an error
        let slot = slot.ok_or(TakeError {
            debug_name: self.debug_name,
        })?;

        Ok(slot)
    }

    pub fn store(&mut self, slot: Box<dyn AnyActorSlot>) -> Result<(), StoreError> {
        // Check if already present
        if self.slot.is_some() {
            return Err(StoreError {
                debug_name: self.debug_name,
            });
        }

        // Put the actor back in the slot
        self.slot = Some(slot);

        Ok(())
    }
}

/// Options to inform the system on how to treat an actor.
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct Options {
    /// Sets if this actor's messages are 'high-priority'.
    ///
    /// Typically, this means the system will always place the actor at the *start* of the queue.
    /// This is useful for actors that simply relay messages to other systems.
    /// In those cases, the message waiting at the end of the queue would hurt performance by
    /// fragmenting batches, increase latency drastically.
    pub high_priority: bool,
}

impl Options {
    /// Convenience alias for `Self::default().with_high_priority()`.
    pub fn high_priority() -> Self {
        Self::default().with_high_priority()
    }

    /// Sets `high_priority` to true.
    pub fn with_high_priority(mut self) -> Self {
        self.high_priority = true;
        self
    }
}

#[derive(Error, Debug)]
#[error("failed to take actor, \"{debug_name}\" is not currently available")]
pub struct TakeError {
    debug_name: &'static str,
}

#[derive(Error, Debug)]
#[error("failed to store actor, \"{debug_name}\" is already in slot")]
pub struct StoreError {
    debug_name: &'static str,
}
