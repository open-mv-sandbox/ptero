use std::collections::VecDeque;

use anyhow::Error;
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
}

impl System {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an actor.
    ///
    /// The actor is started *immediately* in-place, and added to the system.
    pub fn start<F, A>(
        &mut self,
        log_id: &'static str,
        start: F,
    ) -> Result<Addr<A::Family>, StartError>
    where
        F: FnOnce(&mut System, Addr<A::Family>) -> Result<A, Error>,
        A: Actor + 'static,
    {
        // Allocate an address for the actor
        let addr_entry = AddrEntry { log_id, slot: None };
        let index = self.addresses.insert(addr_entry);
        let addr = Addr::from_index(index);

        // Create a tracing span for the actor
        let span = span!(Level::ERROR, "actor", id = log_id);
        let enter = span.enter();
        event!(Level::INFO, "starting actor");

        // Run the actor factory
        let result = (start)(self, addr);

        // Handle factory result
        let actor = result.map_err::<StartError, _>(|source| {
            event!(Level::DEBUG, "actor start failed");
            self.addresses.remove(index);
            StartError::StartCallError { log_id, source }
        })?;

        // Replace the placeholder
        let addr_entry = self
            .addresses
            .get_mut(index)
            .ok_or(InternalError::CorruptActorsState)?;
        drop(enter);
        let actor_entry = ActorEntry {
            span,
            actor: Box::new(actor),
            queued: false,
        };
        addr_entry.slot = Some(actor_entry);

        Ok(addr)
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
        while let Some(index) = self.queue.pop_front() {
            self.process_at(index)?;
        }

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

struct AddrEntry {
    log_id: &'static str,
    slot: Option<ActorEntry>,
}

struct ActorEntry {
    span: Span,
    actor: Box<dyn AnyActor>,
    queued: bool,
}

#[derive(Error, Debug)]
pub enum StartError {
    #[error("failed to start actor{{id=\"{log_id}\"}}, error in actor's start call")]
    StartCallError { log_id: &'static str, source: Error },
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
