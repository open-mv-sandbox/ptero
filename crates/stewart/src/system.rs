use std::collections::VecDeque;

use anyhow::{anyhow, Context, Error};
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
    deferred: VecDeque<DeferredAction>,
}

impl System {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue starting an actor.
    ///
    /// The actor's address is immediately returned, but will not be valid for sending until after
    /// the actor has actually been started.
    ///
    /// Actors are started in the same order `start` is called. This means if an actor's start
    /// function includes starting more actors, these child actors do not become available for
    /// sending message to, until after the parent actor's start is done.
    ///
    /// TODO: Allow pre-starting children by separating ID allocation and start.
    pub fn start<F, A>(&mut self, id: &'static str, start: F) -> Addr<A::Family>
    where
        F: FnOnce(&mut System, Addr<A::Family>) -> Result<A, Error> + 'static,
        A: Actor + 'static,
    {
        // Allocate an address for the actor
        let index = self.addresses.insert(AddrEntry::Empty);
        let addr = Addr::from_id(index);

        // Wrap the factory into a dynamic one
        let factory = move |system: &mut System| {
            let actor = (start)(system, addr)?;
            let actor: Box<dyn AnyActor> = Box::new(actor);
            Ok(actor)
        };

        // Queue the start
        let factory = Box::new(factory);
        let action = DeferredAction::Start { id, index, factory };
        self.deferred.push_back(action);

        addr
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    pub fn handle<'a, F>(
        &mut self,
        addr: Addr<F>,
        message: impl Into<F::Member<'a>>,
    ) -> Result<(), Error>
    where
        F: Family,
        F::Member<'static>: 'static,
    {
        let index = addr.id();

        let address_entry = self
            .addresses
            .get_mut(index)
            // TODO: What to do with addressing error?
            .ok_or(anyhow!("failed to find actor for address"))?;
        let actor_entry = address_entry
            .as_actor()
            // TODO: What to do with addressing error?
            .ok_or(anyhow!("actor is not currently available"))?;

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
        loop {
            // Run all deferred actions
            while let Some(action) = self.deferred.pop_front() {
                match action {
                    DeferredAction::Start { id, index, factory } => {
                        self.start_immediate(id, index, factory)?
                    }
                }
            }

            // Run the next queue entry
            if let Some(index) = self.queue.pop_front() {
                self.process_at(index)?;
            } else {
                break;
            };
        }

        Ok(())
    }

    fn start_immediate(
        &mut self,
        id: &'static str,
        index: Index,
        factory: StartFactory,
    ) -> Result<(), Error> {
        // Create a tracing span for the actor
        let span = span!(Level::ERROR, "actor", id);
        let enter = span.enter();
        event!(Level::INFO, "starting actor");

        // Run the actor factory
        let result = (factory)(self);
        drop(enter);

        // Handle factory result
        let result = match result {
            Ok(actor) => {
                // Replace the placeholder
                let entry = ActorEntry {
                    span,
                    actor,
                    queued: false,
                };
                self.addresses.insert_at(index, AddrEntry::Actor(entry))
            }
            Err(error) => {
                event!(Level::ERROR, "actor failed to start\n{:?}", error);
                self.addresses.remove(index)
            }
        };

        result.context("address entry unexpectedly disappeared")?;

        Ok(())
    }

    fn process_at(&mut self, index: Index) -> Result<(), Error> {
        let address_entry = self
            .addresses
            .get_mut(index)
            .ok_or(anyhow!("invalid id in schedule"))?;

        let mut address_entry = std::mem::replace(address_entry, AddrEntry::Empty);
        let actor_entry = address_entry
            .as_actor()
            // TODO: This isn't an internal error and should be better expressed
            .ok_or(anyhow!("actor not currently present"))?;

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
            drop(address_entry);
            self.addresses.remove(index);
            return Ok(());
        }

        // Return the actor
        drop(enter);
        self.addresses.insert_at(index, address_entry);

        Ok(())
    }
}

enum AddrEntry {
    Empty,
    Actor(ActorEntry),
}

impl AddrEntry {
    fn as_actor(&mut self) -> Option<&mut ActorEntry> {
        if let AddrEntry::Actor(actor) = self {
            Some(actor)
        } else {
            None
        }
    }
}

struct ActorEntry {
    pub span: Span,
    pub actor: Box<dyn AnyActor>,
    pub queued: bool,
}

enum DeferredAction {
    Start {
        id: &'static str,
        index: Index,
        factory: StartFactory,
    },
}

type StartFactory = Box<dyn FnOnce(&mut System) -> Result<Box<dyn AnyActor>, Error>>;

#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("unknown internal error, this is a bug in stewart")]
    Internal(#[from] anyhow::Error),
}
