use std::collections::VecDeque;

use anyhow::{Context, Error};
use family::{any::FamilyMember, Family};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

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

    /// Create an address for an actor on the system, to be later started using `start`.
    ///
    /// The address' debugging name is inferred from the current span's name. This name is only
    /// used in logging.
    pub fn create_addr<F>(
        &mut self,
        parent: ActorId,
    ) -> Result<(ActorId, Addr<F>), CreateAddrError> {
        let parent = parent.0;
        let span = Span::current();

        // For convenience, just infer it from the span name, which should generally be right.
        // The one common case where it isn't is if a span is hidden in filtering, or there never
        // was a span, but the next step improves on that issue.
        let mut debug_name = span.metadata().map(|m| m.name()).unwrap_or("unknown");

        // If the new addr has a parent, but we're still in the same span, just use "unknown"
        if let Some(parent) = parent {
            let parent = self
                .addresses
                .get(parent)
                .ok_or(CreateAddrError::ParentDoesNotExist)?;
            if parent.debug_name == debug_name {
                debug_name = "unknown";
            }
        }

        // Continual span is inherited from the create addr callsite
        let addr_entry = AddrEntry {
            debug_name,
            span,
            queued: false,
            actor: None,
        };
        let index = self.addresses.insert(addr_entry);

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(index);

        let id = ActorId(Some(index));
        let addr = Addr::from_index(index);
        Ok((id, addr))
    }

    /// Start an actor on the system, making its address available for handling.
    pub fn start<A>(&mut self, addr: Addr<A::Family>, actor: A) -> Result<(), StartError>
    where
        A: Actor + 'static,
    {
        event!(Level::INFO, "starting actor");

        // Remove pending, starting is what it's pending for
        let index = self
            .pending_start
            .iter()
            .position(|i| *i == addr.index())
            .ok_or(StartError::ActorNotPending)?;
        self.pending_start.remove(index);

        // Retrieve the slot
        let addr_entry = self
            .addresses
            .get_mut(addr.index())
            .ok_or(StartError::ActorNotFound)?;

        // Fill the slot
        let actor = Box::new(actor);
        addr_entry.actor = Some(actor);

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

        let addr_entry = self
            .addresses
            .get_mut(index)
            .ok_or(HandleError::ActorNotFound)?;
        let span = addr_entry.span.clone();
        let actor = addr_entry
            .actor
            .as_mut()
            .ok_or(HandleError::ActorNotAvailable {
                name: addr_entry.debug_name,
            })?;

        // Let the actor reduce the message
        let _enter = span.enter();
        let message = message.into();
        let mut message = Some(FamilyMember::<F>(message));
        let result = actor.reduce(&mut message);

        // Schedule process if necessary
        match result {
            Ok(AfterReduce::Nothing) => {
                // Nothing to do
            }
            Ok(AfterReduce::Process) => {
                if !addr_entry.queued {
                    self.queue.push_back(index);
                    addr_entry.queued = true;
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
        self.step_cleanup()?;

        while let Some(index) = self.queue.pop_front() {
            self.process_at(index)?;

            self.step_cleanup()?;
        }

        Ok(())
    }

    fn step_cleanup(&mut self) -> Result<(), Error> {
        // Clean up actors that didn't start in time, and thus failed
        // Intentionally in reverse order, clean up children before parents
        while let Some(index) = self.pending_start.pop() {
            self.cleanup_pending(index)?;
        }

        Ok(())
    }

    fn cleanup_pending(&mut self, index: Index) -> Result<(), Error> {
        let entry = self
            .addresses
            .remove(index)
            .context("pending actor address doesn't exist")?;

        let _enter = entry.span.enter();
        event!(Level::INFO, "failed to start in time, cleaning up");

        Ok(())
    }

    fn process_at(&mut self, index: Index) -> Result<(), ProcessError> {
        let (span, mut actor) = self.take_for_process(index)?;

        // Perform the actor's process step
        let _entry = span.enter();
        let result = actor.process(self);

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
            drop(actor);
            self.addresses.remove(index);
            return Ok(());
        }

        // Return the actor otherwise
        let addr_entry = self
            .addresses
            .get_mut(index)
            .context("actor disappeared during process")?;
        addr_entry.actor = Some(actor);

        Ok(())
    }

    fn take_for_process(
        &mut self,
        index: Index,
    ) -> Result<(Span, Box<dyn AnyActor>), ProcessError> {
        let addr_entry = self
            .addresses
            .get_mut(index)
            .context("queued actor address doesn't exist")?;

        // Mark the actor as no longer queued, as we're processing it
        addr_entry.queued = false;

        // Take the actor from the slot
        let span = addr_entry.span.clone();
        let actor = std::mem::replace(&mut addr_entry.actor, None);

        // If the actor wasn't in the slot, return an error
        let actor = actor.context("actor not available")?;

        Ok((span, actor))
    }
}

impl Drop for System {
    fn drop(&mut self) {
        let mut names = Vec::new();
        for (_, addr_entry) in self.addresses.drain() {
            names.push(addr_entry.debug_name);
        }

        if !names.is_empty() {
            let names = names.join(",");
            event!(Level::WARN, names, "actors not stopped before system drop");
        }
    }
}

struct AddrEntry {
    /// Non-unqiue debugging name, used to improve logging.
    debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    span: Span,
    queued: bool,
    actor: Option<Box<dyn AnyActor>>,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ActorId(pub(crate) Option<Index>);

impl ActorId {
    /// Address ID of the root of the system.
    ///
    /// You can use this to start actors with no parent other than the system root, which thus
    /// exceed the lifetime of the current actor.
    pub fn root() -> Self {
        Self(None)
    }
}

#[derive(Error, Debug)]
pub enum CreateAddrError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ParentDoesNotExist,
}

#[derive(Error, Debug)]
pub enum StartError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ActorNotPending,
    #[error("failed to start actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to start actor, actor at address already started")]
    ActorAlreadyStarted,
    #[error("internal error, this is a bug in stewart")]
    Internal(#[from] Error),
}

#[derive(Error, Debug)]
pub enum HandleError {
    #[error("failed to handle message, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to handle message, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
}

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("internal error, this is a bug in stewart")]
    Internal(#[from] Error),
}
