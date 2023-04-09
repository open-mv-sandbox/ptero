use std::{
    collections::VecDeque,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicPtr,
};

use anyhow::{Context as AContext, Error};
use tracing::{event, Level};

use crate::{actor_tree::ActorTree, Actor, AddrError, After, CreateError, Id, Options, StartError};

/// Thread-local actor execution system.
#[derive(Default)]
pub struct System {
    actors: ActorTree,
    queue: VecDeque<Id>,
}

impl System {
    /// Create a new empty `System`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the root context.
    pub fn root(&mut self) -> Context {
        Context {
            system: self,
            current: None,
        }
    }

    /// Send a message to an actor.
    ///
    /// This will never be handled in-place. The system will queue up the message to be processed
    /// at a later time.
    pub fn send<M>(&mut self, addr: Addr<M>, message: impl Into<M>)
    where
        M: 'static,
    {
        let result = self.try_send(addr.id, message.into());

        // TODO: Figure out what to do with this, it may be useful to have a unified "send error"
        // system, but in some cases a definitive error may never happen until the implementor
        // decides that it's too long?
        // Some cases, it's an error with the receiver, some cases it's an error with the sender.
        // This needs more thought before making an API decision.
        if let Err(error) = result {
            event!(Level::ERROR, ?error, "failed to send message");
        }
    }

    fn try_send<M>(&mut self, id: Id, message: M) -> Result<(), Error>
    where
        M: 'static,
    {
        let node = self.actors.get_mut(id).context("actor not found")?;
        let slot = node.slot_mut().context("actor not available")?;

        let mut message = Some(message);
        slot.handle(&mut message)?;

        // Queue for later processing
        if !self.queue.contains(&id) {
            if !node.options().high_priority {
                self.queue.push_back(id);
            } else {
                self.queue.push_front(id);
            }
        }

        Ok(())
    }

    /// Process all pending messages, until none are left.
    ///
    /// Processing messages may create more messages, so this is not guaranteed to ever return.
    /// However, well-behaved actors avoid should behave appropriately for the kind of system
    /// they're running on. For example, IO actors shouldn't keep the system busy, preventing it
    /// from handling IO reactor messages.
    pub fn run_until_idle(&mut self) -> Result<(), Error> {
        self.actors.cleanup_pending()?;

        while let Some(index) = self.queue.pop_front() {
            self.process(index).context("failed to process")?;

            self.actors.cleanup_pending()?;
        }

        Ok(())
    }

    fn process(&mut self, id: Id) -> Result<(), Error> {
        // Borrow the actor
        let node = self
            .actors
            .get_mut(id)
            .context("failed to get actor before process")?;
        let span = node.span();
        let _enter = span.enter();
        let mut slot = node.take()?;

        // Run the actor's handler
        let mut ctx = Context {
            system: self,
            current: Some(id),
        };
        let after = slot.process(&mut ctx);

        // If we got told to stop the actor, do that instead of returning
        if after == After::Stop {
            event!(Level::INFO, "stopping actor");
            drop(slot);
            self.actors.remove(id)?;
            return Ok(());
        }

        // Return the actor
        self.actors
            .get_mut(id)
            .context("failed to get actor after process")?
            .store(slot)?;

        Ok(())
    }
}

impl Drop for System {
    fn drop(&mut self) {
        let debug_names = self.actors.debug_names();

        if !debug_names.is_empty() {
            event!(
                Level::WARN,
                ?debug_names,
                "actors not stopped before system drop"
            );
        }
    }
}

/// Typed system address of an actor, used for sending messages to the actor.
///
/// This address can only be used with one specific system. Using it with another system is
/// not unsafe, but may result in unexpected behavior.
///
/// When distributing work between systems, you can use an 'envoy' actor that relays messages from
/// one system to another. For example, using an MPSC channel, or even across network.
pub struct Addr<M> {
    id: Id,
    _m: PhantomData<AtomicPtr<M>>,
}

impl<M> Clone for Addr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for Addr<M> {}

pub struct Context<'a> {
    system: &'a mut System,
    current: Option<Id>,
}

impl<'a> Context<'a> {
    /// Get the address of the current actor.
    pub fn addr<M>(&self) -> Result<Addr<M>, AddrError> {
        // TODO: Early message type validation?

        let id = self.current.ok_or(AddrError::Root)?;

        let addr = Addr {
            id,
            _m: PhantomData,
        };
        Ok(addr)
    }

    /// Create a new actor.
    ///
    /// The actor's address will not be available for handling messages until `start` is called.
    pub fn create(&mut self) -> Result<Context, CreateError> {
        event!(Level::INFO, "creating actor");

        let id = self.system.actors.create(self.current)?;

        let ctx = Context {
            system: self.system,
            current: Some(id),
        };
        Ok(ctx)
    }

    /// Start an actor, making it available for handling messages.
    pub fn start<A>(&mut self, options: Options, actor: A) -> Result<(), StartError>
    where
        A: Actor,
    {
        event!(Level::INFO, "starting actor");

        let id = self.current.ok_or(StartError::Root)?;
        self.system.actors.start(id, options, actor)?;

        Ok(())
    }
}

impl<'a> Deref for Context<'a> {
    type Target = System;

    fn deref(&self) -> &System {
        &self.system
    }
}

impl<'a> DerefMut for Context<'a> {
    fn deref_mut(&mut self) -> &mut System {
        &mut self.system
    }
}
