use std::collections::VecDeque;

use anyhow::{bail, Context, Error};
use family::{any::FamilyMember, Family};
use thunderdome::{Arena, Index};
use tracing::{event, span, Level, Span};

use crate::{dynamic::AnyActor, Actor, ActorAddr, AfterProcess, AfterReduce};

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
    pub fn start<F, A>(&mut self, id: &'static str, start: F)
    where
        F: FnOnce(&mut System, ActorAddr<A::Family>) -> Result<A, Error> + 'static,
        A: Actor + 'static,
    {
        let start = move |system: &mut System, index: Index| {
            // Create a tracing span for the actor
            let span = span!(Level::INFO, "actor", id);
            let entry = span.enter();

            // Convert the index to an addr
            let addr = ActorAddr::from_id(index);

            // Run the starting function
            event!(Level::TRACE, "starting actor");
            let actor = (start)(system, addr)?;
            let actor: Box<dyn AnyActor> = Box::new(actor);

            drop(entry);
            Ok((span, actor))
        };

        let action = DeferredAction::Start(Box::new(start));
        self.deferred.push(action);
    }

    /// Handle a message, immediately sending it to the actor's reducer.
    pub fn handle<'a, F>(&mut self, addr: ActorAddr<F>, message: impl Into<F::Member<'a>>)
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

        let message = message.into();
        let mut message = Some(FamilyMember::<F>(message));
        let result = entry.actor.reduce(&mut message);

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
        start: DeferredStart,
        dummy_entry: &mut Option<ActorEntry>,
    ) -> Result<(), Error> {
        // Get an index for the actor by starting a dummy actor
        let dummy_entry_val = dummy_entry.take().context("dummy entry already taken")?;
        let index = self.actors.insert(dummy_entry_val);

        // Start the real actor
        let result = (start)(self, index);

        // Handle factory result
        let result = match result {
            Ok((span, actor)) => {
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

pub struct ActorEntry {
    pub span: Span,
    pub actor: Box<dyn AnyActor>,
    pub queued: bool,
}

pub enum DeferredAction {
    Start(DeferredStart),
}

pub type DeferredStart =
    Box<dyn FnOnce(&mut System, Index) -> Result<(Span, Box<dyn AnyActor>), Error>>;
