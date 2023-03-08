use std::collections::VecDeque;

use anyhow::{Context, Error};
use family::{any::FamilyMember, Family};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{dynamic::AnyActor, Actor, Addr, AfterProcess, AfterReduce, Id, Info};

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
    pub fn create_actor<A: Actor>(&mut self, parent: Id) -> Result<Info<A>, CreateAddrError> {
        let _parent = parent.0;
        let span = Span::current();

        // Continual span is inherited from the create addr callsite
        let addr_entry = AddrEntry {
            debug_name: debug_name::<A>(),
            span,
            queued: false,
            actor: None,
        };
        let index = self.addresses.insert(addr_entry);

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(index);

        Ok(Info::new(index))
    }

    /// Start an actor on the system, making its address available for handling.
    pub fn start_actor<A>(&mut self, info: Info<A>, actor: A) -> Result<(), StartError>
    where
        A: Actor + 'static,
    {
        event!(Level::INFO, "starting actor");

        // Remove pending, starting is what it's pending for
        let index = self
            .pending_start
            .iter()
            .position(|i| *i == info.index())
            .ok_or(StartError::ActorNotPending)?;
        self.pending_start.remove(index);

        // Retrieve the slot
        let addr_entry = self
            .addresses
            .get_mut(info.index())
            .ok_or(StartError::ActorNotFound)?;

        // Fill the slot
        let actor = Box::new(actor);
        addr_entry.actor = Some(actor);

        Ok(())
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    ///
    /// Handle never returns an error, but is not guaranteed to actually deliver the message.
    /// Message delivery failure can be for a variety of reasons, not always caused by the sender,
    /// and not always caused by the receiver. This makes it unclear who should receive the error.
    ///
    /// The error is logged, and may in the future be handleable. If you have a use case where you
    /// need to handle a handle error, open an issue.
    pub fn handle<'a, F>(&mut self, addr: Addr<F>, message: impl Into<F::Member<'a>>)
    where
        F: Family,
        F::Member<'static>: 'static,
    {
        let result = self.try_handle(addr, message);
        match result {
            Ok(value) => value,
            Err(error) => {
                event!(Level::WARN, "failed to handle message\n{:?}", error);
            }
        }
    }

    fn try_handle<'a, F>(
        &mut self,
        addr: Addr<F>,
        message: impl Into<F::Member<'a>>,
    ) -> Result<(), Error>
    where
        F: Family,
        F::Member<'static>: 'static,
    {
        // Attempt to borrow the actor for handling
        let (entry, mut actor) = self.borrow(addr.index())?;

        // Enter the actor's span for logging
        let span = entry.span.clone();
        let _entry = span.enter();

        // Let the actor reduce the message
        let message = message.into();
        let mut message = Some(FamilyMember::<F>(message));
        let result = actor.reduce(self, &mut message);

        // Handle the result
        let after = match result {
            Ok(value) => value,
            Err(error) => {
                // TODO: What to do with this?
                event!(Level::ERROR, "actor failed to reduce message\n{:?}", error);
                AfterReduce::Nothing
            }
        };

        // Return the actor
        let entry = self.unborrow(addr.index(), actor)?;

        // Schedule process if necessary
        if after == AfterReduce::Process {
            if !entry.queued {
                entry.queued = true;
                self.queue.push_back(addr.index());
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
        let (entry, mut actor) = self.borrow(index)?;

        // Mark the actor as no longer queued, as we're processing it
        entry.queued = false;

        // Enter the actor's span for logging
        let span = entry.span.clone();
        let _entry = span.enter();

        // Perform the actor's process step
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
        } else {
            // Return the actor otherwise
            self.unborrow(index, actor)?;
        }

        Ok(())
    }

    fn borrow(&mut self, index: Index) -> Result<(&mut AddrEntry, Box<dyn AnyActor>), BorrowError> {
        // Find the actor's entry
        let entry = self
            .addresses
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

    fn unborrow(
        &mut self,
        index: Index,
        actor: Box<dyn AnyActor>,
    ) -> Result<&mut AddrEntry, BorrowError> {
        let entry = self
            .addresses
            .get_mut(index)
            .ok_or(BorrowError::InternalActorDisappeared)?;
        entry.actor = Some(actor);

        Ok(entry)
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
    /// Debugging identification name, not intended for anything other than warn/err reporting.
    debug_name: &'static str,
    /// Persistent logging span, groups logging that happenened under this actor.
    span: Span,
    queued: bool,
    actor: Option<Box<dyn AnyActor>>,
}

fn debug_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    let before_generics = name.split("<").next().unwrap_or("Unknown");
    let after_modules = before_generics.split("::").last().unwrap_or("Unknown");
    after_modules
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
pub enum ProcessError {
    #[error("failed to process actor, borrow error")]
    BorrowError(#[from] BorrowError),
    #[error("internal error, this is a bug in stewart")]
    Internal(#[from] Error),
}

#[derive(Error, Debug)]
pub enum BorrowError {
    #[error("failed to borrow actor, no actor exists at the given address")]
    ActorNotFound,
    #[error("failed to borrow actor, actor ({name}) at the address exists, but is not currently available")]
    ActorNotAvailable { name: &'static str },
    #[error("this is a bug in stewart, the actor disappeared before it could be returned")]
    InternalActorDisappeared,
}
