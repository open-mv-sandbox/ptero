use std::collections::VecDeque;

use anyhow::{bail, Context, Error};
use family::{AnyOptionMut, Family};
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{
    dynamic::AnyActor,
    factory::{AnyFactory, Factory},
    ActorAddr, AfterProcess, AfterReduce, Start,
};

/// Thread-local cooperative multitasking actor scheduler.
///
/// This executor bridges CPU threads into cooperative actor threads.
/// It does not do any scheduling in itself, this is delegated to an actor.
#[derive(Default)]
pub struct System {
    actors: Arena<ActorEntry>,
    queue: VecDeque<Index>,
    deferred: Vec<DeferredAction>,
}

impl System {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue starting an actor.
    pub fn start<S>(&mut self, data: S::Data)
    where
        S: Start + 'static,
    {
        let factory = Factory::<S>::new(data);
        let action = DeferredAction::Start(Box::new(factory));
        self.deferred.push(action);
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    pub fn handle<'a, F>(&mut self, addr: ActorAddr<F>, message: F::Member<'a>)
    where
        F: Family,
        F::Member<'static>: 'static,
    {
        let index = addr.id();

        let entry = match self.actors.get_mut(index) {
            Some(actor) => actor,
            None => {
                // TODO: What to do with addressing error?
                event!(Level::ERROR, "failed to find actor for system address");
                return;
            }
        };

        // Let the actor reduce the message
        let enter = entry.span.enter();

        let mut message_slot = Some(message);
        let slot = AnyOptionMut::new::<F>(&mut message_slot);
        let result = entry.actor.reduce(slot);

        // Schedule process if necessary
        match result {
            Ok(AfterReduce::Nothing) => {
                // Nothing to do
            }
            Ok(AfterReduce::Process) => {
                if !entry.queued {
                    self.queue.push_back(index);
                    entry.queued = true;
                }
            }
            Err(error) => {
                // TODO: What to do with this?
                event!(Level::ERROR, "actor failed to reduce message\n{:?}", error);
            }
        }

        drop(enter);
    }

    pub(crate) fn queue_next(&mut self) -> Option<Index> {
        self.queue.pop_front()
    }

    pub(crate) fn deferred_next(&mut self) -> Option<DeferredAction> {
        self.deferred.pop()
    }

    pub(crate) fn start_immediate(
        &mut self,
        factory: Box<dyn AnyFactory>,
        dummy_entry: &mut Option<ActorEntry>,
    ) -> Result<(), Error> {
        let span = factory.create_span();
        let entry = span.enter();
        event!(Level::TRACE, "starting actor");

        // Get an index for the actor by starting a dummy actor
        let dummy_entry_val = dummy_entry.take().context("dummy entry already taken")?;
        let index = self.actors.insert(dummy_entry_val);

        // Start the real actor
        let result = factory.start(self, index);

        drop(entry);

        // Handle factory result
        let result = match result {
            Ok(actor) => {
                // Replace the placeholder
                let entry = ActorEntry {
                    span,
                    actor,
                    queued: false,
                };
                self.actors.insert_at(index, entry)
            }
            Err(error) => {
                event!(Level::ERROR, "actor failed to start\n{:?}", error);
                self.actors.remove(index)
            }
        };

        let dummy_entry_val = result.context("actor unexpectedly disappeared")?;
        *dummy_entry = Some(dummy_entry_val);

        Ok(())
    }

    pub(crate) fn process_at(
        &mut self,
        index: Index,
        dummy_entry: &mut Option<ActorEntry>,
    ) -> Result<(), Error> {
        if !self.actors.contains(index) {
            bail!("invalid id in schedule");
        }

        // Swap out for a dummy actor
        let dummy_entry_val = dummy_entry.take().context("dummy entry already taken")?;
        let mut entry = self
            .actors
            .insert_at(index, dummy_entry_val)
            .context("actor unexpectedly disappeared")?;

        // Perform the actor's process step
        let enter = entry.span.enter();

        let result = entry.actor.process(self);
        entry.queued = false;

        drop(enter);

        // Re-insert the actor
        let dummy_entry_val = self
            .actors
            .insert_at(index, entry)
            .context("actor unexpectedly disappeared")?;
        *dummy_entry = Some(dummy_entry_val);

        // Handle the result
        match result {
            Ok(AfterProcess::Nothing) => {
                // Nothing to do
            }
            Ok(AfterProcess::Stop) => {
                self.stop(index)?;
            }
            Err(error) => {
                bail!("actor failed to process\n{:?}", error);
            }
        }

        Ok(())
    }

    fn stop(&mut self, index: Index) -> Result<(), Error> {
        let entry = self.actors.remove(index).context("actor didn't exist")?;
        let _entry = entry.span.enter();
        event!(Level::TRACE, "stopping actor");

        Ok(())
    }
}

pub(crate) struct ActorEntry {
    pub span: Span,
    pub actor: Box<dyn AnyActor>,
    pub queued: bool,
}

pub(crate) enum DeferredAction {
    Start(Box<dyn AnyFactory>),
}
