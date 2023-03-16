use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{node::Node, slot::AnyActorSlot, Options};

#[derive(Default)]
pub struct ActorTree {
    nodes: Arena<Node>,
    pending_start: Vec<Index>,
}

impl ActorTree {
    pub fn create<A>(&mut self, parent: Option<Index>) -> Result<Index, CreateActorError> {
        // Continual span is inherited from the create addr callsite
        let span = Span::current();
        let debug_name = debug_name::<A>();

        // Link to the parent
        if let Some(parent) = parent {
            self.nodes
                .get_mut(parent)
                .ok_or(CreateActorError::ParentDoesNotExist)?;
        }

        // Create the node
        let node = Node::new(debug_name, span);
        let index = self.nodes.insert(node);

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(index);

        Ok(index)
    }

    pub fn start(
        &mut self,
        index: Index,
        options: Options,
        slot: Box<dyn AnyActorSlot>,
    ) -> Result<(), StartActorError> {
        // Check if it's pending, and remove it
        let pending_index = self
            .pending_start
            .iter()
            .position(|i| *i == index)
            .ok_or(StartActorError::ActorStarted)?;
        self.pending_start.remove(pending_index);

        // Apply the start
        let node = self.get_mut(index).context("failed to get node")?;
        node.set_options(options);
        node.store(slot).context("failed to store actor")?;

        Ok(())
    }

    /// Clean up actors that didn't start in time, and thus failed.
    pub fn cleanup_pending(&mut self) -> Result<(), Error> {
        // Intentionally in reverse order, clean up children before parents
        while let Some(index) = self.pending_start.pop() {
            self.cleanup_pending_at(index)?;
        }

        Ok(())
    }

    fn cleanup_pending_at(&mut self, index: Index) -> Result<(), Error> {
        let node = self.remove(index)?;

        let span = node.span();
        let _enter = span.enter();
        event!(Level::INFO, "actor failed to start in time");

        Ok(())
    }

    pub fn get_mut(&mut self, index: Index) -> Option<&mut Node> {
        self.nodes.get_mut(index)
    }

    pub fn remove(&mut self, index: Index) -> Result<Node, Error> {
        self.nodes.remove(index).context("actor doesn't exist")
    }

    /// Get the debug names of all active actors, except root.
    pub fn debug_names(&self) -> Vec<&'static str> {
        let mut debug_names = Vec::new();

        for (_, node) in &self.nodes {
            debug_names.push(node.debug_name());
        }

        debug_names
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CreateActorError {
    #[error("failed to start actor, actor isn't pending to be started")]
    ParentDoesNotExist,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StartActorError {
    #[error("failed to start actor, actor already started")]
    ActorStarted,
    #[error("failed to start actor, internal error")]
    Internal(#[from] Error),
}

fn debug_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    let before_generics = name.split('<').next().unwrap_or("Unknown");
    let after_modules = before_generics.split("::").last().unwrap_or("Unknown");
    after_modules
}
