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
    pub fn new(debug_name: &'static str, span: Span) -> Self {
        Self {
            debug_name,
            span,
            options: Options::default(),
            slot: None,
        }
    }

    pub fn debug_name(&self) -> &'static str {
        self.debug_name
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
pub struct Options {
    pub(crate) high_priority: bool,
}

impl Options {
    /// Set this actor's messages to be high-priority.
    ///
    /// Typically, this means the system will always place the actor at the *start* of the queue.
    /// This is useful for actors that simply relay messages to other systems, where the message
    /// waiting at the end of the queue would hurt performance, and increase latency drastically.
    pub fn high_priority(mut self) -> Self {
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
