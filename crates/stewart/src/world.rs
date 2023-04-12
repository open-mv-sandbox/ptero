use std::{
    collections::{HashMap, VecDeque},
    marker::PhantomData,
    sync::atomic::AtomicPtr,
};

use anyhow::{Context, Error};
use thunderdome::{Arena, Index};
use tracing::{event, Level};

use crate::{
    system::{AnySystemEntry, SystemEntry},
    tree::{Node, Tree},
    CreateError, System, SystemOptions,
};

/// Thread-local system and actor collection.
#[derive(Default)]
pub struct World {
    systems: Arena<SystemSlot>,
    tree: Tree,
    queue: VecDeque<SystemId>,

    pending_stop: Vec<(ActorId, SystemId)>,
}

impl World {
    /// Create a new empty `System`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an actor processing system.
    ///
    /// You are recommended to re-use these for actors of the same type. Processing many messages
    /// at once is generally faster, and allows you to perform additional optimizations.
    /// However, the overhead of not re-using systems is acceptable in most cases.
    ///
    /// TODO: Option for automatic re-use by scanning for a system of a type to already exist.
    pub fn register<S>(&mut self, options: SystemOptions, system: S) -> SystemId
    where
        S: System,
    {
        let entry = Box::new(SystemEntry::new(system));
        let slot = SystemSlot {
            options,
            entry: Some(entry),
        };
        let index = self.systems.insert(slot);

        SystemId { index }
    }

    /// Create a new actor.
    ///
    /// The actor's address will not be available for handling messages until `start` is called.
    pub fn create(
        &mut self,
        system: SystemId,
        parent: Option<ActorId>,
    ) -> Result<ActorId, CreateError> {
        event!(Level::INFO, "creating actor");

        let node = Node::new(system, parent.map(|i| i.index));
        let index = self.tree.insert(node)?;

        Ok(ActorId { index })
    }

    /// Start an actor instance, making it available for handling messages.
    pub fn start<I>(&mut self, actor: ActorId, instance: I) -> Result<(), Error>
    where
        I: 'static,
    {
        event!(Level::INFO, "starting actor");

        // TODO: Validate not started yet
        // TODO: Typed errors once again

        // Find the node for the actor, and the associated system
        let node = self
            .tree
            .get_mut(actor.index)
            .context("failed to find node")?;
        let slot = self
            .systems
            .get_mut(node.system().index)
            .context("failed to find system")?;
        let system = slot.entry.as_mut().context("system unavailable")?;

        // Give the instance to the system
        let mut instance = Some(instance);
        system.insert(actor, &mut instance);

        Ok(())
    }

    /// Stop an actor immediately, and queue it for removal from systems later.
    ///
    /// After stopping an actor will no longer accept messages, but can still process them.
    /// After the current process step is done, the actor and all remaining pending messages will
    /// be dropped.
    pub fn stop(&mut self, actor: ActorId) -> Result<(), Error> {
        let pending_stop = &mut self.pending_stop;

        // Ignore already pending to stop
        if pending_stop.iter().any(|(i, _)| *i == actor) {
            return Ok(());
        }

        // Remove from the tree and mark any removed nodes as pending to stop
        self.tree.remove(actor.index, |node| {
            pending_stop.push((actor, node.system()));
        })?;

        Ok(())
    }

    /// Send a message to an actor.
    ///
    /// This will never be handled in-place. The system will queue up the message to be processed
    /// at a later time.
    pub fn send<M>(&mut self, addr: Addr<M>, message: impl Into<M>)
    where
        M: 'static,
    {
        let result = self.try_send(addr.actor, message.into());

        // TODO: What to do with this error?
        if let Err(error) = result {
            event!(Level::ERROR, ?error, "failed to send message");
        }
    }

    fn try_send<M>(&mut self, actor: ActorId, message: M) -> Result<(), Error>
    where
        M: 'static,
    {
        let node = self.tree.get(actor.index).context("failed to find node")?;

        // Find the system associated with this node
        let system_id = node.system();
        let slot = self
            .systems
            .get_mut(system_id.index)
            .context("failed to find system")?;
        let system = slot.entry.as_mut().context("system unavailable")?;

        // Hand the message to the system
        let mut message = Some(message);
        system.enqueue(actor, &mut message);

        // Queue for later processing
        if !self.queue.contains(&system_id) {
            if !slot.options.high_priority {
                self.queue.push_back(system_id);
            } else {
                self.queue.push_front(system_id);
            }
        }

        Ok(())
    }

    /// Process all pending messages, until none are left.
    pub fn run_until_idle(&mut self) -> Result<(), Error> {
        self.apply_pending().context("failed to apply pending")?;

        while let Some(system_id) = self.queue.pop_front() {
            self.process(system_id).context("failed to process")?;

            self.apply_pending().context("failed to apply pending")?;
        }

        Ok(())
    }

    fn process(&mut self, system_id: SystemId) -> Result<(), Error> {
        // Borrow the system
        let slot = self
            .systems
            .get_mut(system_id.index)
            .context("failed to find system")?;
        let mut system = slot.entry.take().context("system unavailable")?;

        // Run the process handler
        system.process(self);

        // Return the system
        let slot = self
            .systems
            .get_mut(system_id.index)
            .context("failed to find system for return")?;
        slot.entry = Some(system);

        Ok(())
    }

    fn apply_pending(&mut self) -> Result<(), Error> {
        // TODO: Check pending start

        // Finalize stopping
        for (actor, system) in self.pending_stop.drain(..) {
            let slot = self
                .systems
                .get_mut(system.index)
                .context("failed to find system")?;
            let system = slot.entry.as_mut().context("system unavailable")?;

            system.remove(actor);
        }

        Ok(())
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let counts = self.tree.count();

        if !counts.is_empty() {
            let counts: HashMap<_, _> = counts
                .into_iter()
                .map(|(system, count)| {
                    let name = self
                        .systems
                        .get(system.index)
                        .and_then(|s| s.entry.as_ref())
                        .map(|s| s.debug_name())
                        .unwrap_or("Unknown");
                    (name, count)
                })
                .collect();

            event!(
                Level::WARN,
                "actors not stopped before system drop\n{:#?}",
                counts
            );
        }
    }
}

struct SystemSlot {
    options: SystemOptions,
    entry: Option<Box<dyn AnySystemEntry>>,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ActorId {
    index: Index,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct SystemId {
    index: Index,
}

/// Typed system address of an actor, used for sending messages to the actor.
///
/// This address can only be used with one specific system. Using it with another system is
/// not unsafe, but may result in unexpected behavior.
///
/// When distributing work between systems, you can use an 'envoy' actor that relays messages from
/// one system to another. For example, using an MPSC channel, or even across network.
pub struct Addr<M> {
    actor: ActorId,
    _m: PhantomData<AtomicPtr<M>>,
}

impl<M> Addr<M> {
    pub fn new(actor: ActorId) -> Self {
        Self {
            actor,
            _m: PhantomData,
        }
    }
}

impl<M> Clone for Addr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for Addr<M> {}
