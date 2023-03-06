use std::collections::VecDeque;

use family::{any::FamilyMember, Family};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, span, Level, Span};

use crate::{dynamic::AnyActor, Actor, Addr, AfterProcess, AfterReduce};

/// Thread-local cooperative multitasking actor scheduler.
///
/// This executor bridges CPU threads into cooperative actor threads.
/// It does not do any scheduling in itself, this is delegated to an actor.
#[derive(Default)]
pub struct System {
    addresses: Arena<AddrEntry>,
    queue: VecDeque<Index>,
    pending_start: Vec<Index>,
}

impl System {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a slot for an actor on the system, to be later started.
    pub fn create<F>(&mut self, log_id: &'static str) -> Addr<F> {
        let addr_entry = AddrEntry { log_id, slot: None };
        let index = self.addresses.insert(addr_entry);
        self.pending_start.push(index);
        Addr::from_index(index)
    }

    /// Start an actor on the system, making its address available for handling.
    pub fn start<A>(&mut self, addr: Addr<A::Family>, actor: A) -> Result<(), StartError>
    where
        A: Actor + 'static,
    {
        // Remove pending, starting is what it's pending for
        self.pending_start.retain(|v| *v != addr.index());

        // Retrieve the slot
        let addr_entry = self
            .addresses
            .get_mut(addr.index())
            .ok_or(StartError::ActorNotFound)?;

        // Replace the placeholder
        let span = span!(Level::ERROR, "actor", id = addr_entry.log_id);
        let actor_entry = ActiveActor {
            span,
            actor: Box::new(actor),
            queued: false,
        };
        addr_entry.slot = Some(actor_entry);

        Ok(())
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    pub fn handle<'a, F>(
        &mut self,
        addr: Addr<F>,
        message: impl Into<F::Member<'a>>,
    ) -> Result<(), HandleError>
    where
        F: Family,
        F::Member<'static>: 'static,
    {
        let index = addr.index();

        let address_entry = self
            .addresses
            .get_mut(index)
            .ok_or(HandleError::ActorNotFound)?;
        let actor_entry = address_entry
            .slot
            .as_mut()
            .ok_or(HandleError::ActorNotAvailable {
                log_id: address_entry.log_id,
            })?;

        // Let the actor reduce the message
        let _enter = actor_entry.span.enter();

        let message = message.into();
        let mut message = Some(FamilyMember::<F>(message));
        let result = actor_entry.actor.reduce(&mut message);

        // Schedule process if necessary
        match result {
            Ok(AfterReduce::Nothing) => {
                // Nothing to do
            }
            Ok(AfterReduce::Process) => {
                if !actor_entry.queued {
                    self.queue.push_back(index);
                    actor_entry.queued = true;
                }
            }
            Err(error) => {
                // TODO: What to do with this?
                event!(Level::ERROR, "actor failed to reduce message\n{:?}", error);
            }
        }

        Ok(())
    }

    /// Run all queued actor processing tasks, until none remain.
    ///
    /// Running a process task may spawn new process tasks, so this is not guaranteed to ever
    /// return.
    pub fn run_until_idle(&mut self) -> Result<(), ProcessError> {
        self.cleanup()?;

        while let Some(index) = self.queue.pop_front() {
            self.process_at(index)?;

            self.cleanup()?;
        }

        Ok(())
    }

    fn cleanup(&mut self) -> Result<(), InternalError> {
        // Clean up actors that didn't start in time, and thus failed
        // Intentionally in reverse order, clean up children before parents
        while let Some(index) = self.pending_start.pop() {
            self.cleanup_pending(index)?;
        }

        Ok(())
    }

    fn cleanup_pending(&mut self, index: Index) -> Result<(), InternalError> {
        let entry = self
            .addresses
            .remove(index)
            .ok_or(InternalError::CorruptActorsState)?;

        let span = span!(Level::ERROR, "actor", id = entry.log_id);
        let _enter = span.enter();
        event!(Level::INFO, "failed to start in time, cleaning up");
        drop(entry);

        Ok(())
    }

    fn process_at(&mut self, index: Index) -> Result<(), ProcessError> {
        let addr_entry = self
            .addresses
            .get_mut(index)
            .ok_or(InternalError::CorruptQueueState)?;

        let state = std::mem::replace(&mut addr_entry.slot, None);
        let mut actor_entry = state.ok_or(ProcessError::ActorNotAvailable {
            log_id: addr_entry.log_id,
        })?;

        // Perform the actor's process step
        let enter = actor_entry.span.enter();

        let result = actor_entry.actor.process(self);
        actor_entry.queued = false;

        // Handle the result
        let after = match result {
            Ok(after) => after,
            Err(error) => {
                event!(Level::ERROR, "actor failed to process\n{:?}", error);
                AfterProcess::Nothing
            }
        };

        // Stop the actor if we have to
        if after == AfterProcess::Stop {
            event!(Level::INFO, "stopping actor");
            drop(enter);
            drop(actor_entry);
            self.addresses.remove(index);
            return Ok(());
        }

        // Return the actor
        let addr_entry = self
            .addresses
            .get_mut(index)
            .ok_or(InternalError::CorruptActorsState)?;
        drop(enter);
        addr_entry.slot = Some(actor_entry);

        Ok(())
    }
}

impl Drop for System {
    fn drop(&mut self) {
        let mut ids = Vec::new();
        for actor in self.addresses.drain() {
            ids.push(actor.1.log_id);
        }

        if !ids.is_empty() {
            let ids = ids.join(",");
            event!(Level::WARN, ids, "actors not stopped before system drop");
        }
    }
}

struct AddrEntry {
    log_id: &'static str,
    slot: Option<ActiveActor>,
}

struct ActiveActor {
    /// Continual span of the active actor.
    span: Span,
    actor: Box<dyn AnyActor>,
    queued: bool,
}

#[derive(Error, Debug)]
pub enum StartError {
    #[error("failed to start actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to start actor, actor at address already started")]
    ActorAlreadyStarted,
    #[error("internal error, this is a bug in stewart")]
    Internal(#[from] InternalError),
}

#[derive(Error, Debug)]
pub enum HandleError {
    #[error("failed to handle message, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to handle message, actor{{id=\"{log_id}\"}} at the address exists, but is not currently available")]
    ActorNotAvailable { log_id: &'static str },
}

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("failed to process actor, actor{{id=\"{log_id}\"}} at the address exists, but is not currently available")]
    ActorNotAvailable { log_id: &'static str },
    #[error("internal error, this is a bug in stewart")]
    Internal(#[from] InternalError),
}

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("internal actor state was corrupt")]
    CorruptActorsState,
    #[error("internal queue state was corrupt")]
    CorruptQueueState,
}
