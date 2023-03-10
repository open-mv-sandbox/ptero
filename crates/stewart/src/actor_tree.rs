use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{
    node::{BorrowError, Node},
    slot::AnyActorSlot,
    Options,
};

pub struct ActorTree {
    nodes: Arena<Node>,
    pending_start: Vec<Index>,
    root: Index,
}

impl ActorTree {
    pub fn new() -> Self {
        let mut actors = Arena::new();

        // Insert a no-op root actor for tracking purposes
        let actor = Node::new("Root", Span::current());
        let root = actors.insert(actor);

        Self {
            nodes: actors,
            pending_start: Vec::new(),
            root,
        }
    }

    pub fn root(&self) -> Index {
        self.root
    }

    pub fn create<A>(&mut self, parent: Index) -> Result<Index, CreateActorError> {
        // Continual span is inherited from the create addr callsite
        let span = Span::current();
        let debug_name = debug_name::<A>();

        // Link to the parent
        self.nodes
            .get_mut(parent)
            .ok_or(CreateActorError::ParentDoesNotExist)?;

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
        // Remove pending, starting is what it's pending for
        let pending_index = self
            .pending_start
            .iter()
            .position(|i| *i == index)
            .ok_or(StartActorError::ActorNotPending)?;
        self.pending_start.remove(pending_index);

        let node = self.get_mut(index)?;
        node.options = options;
        node.return_slot(slot)?;

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

        let _enter = node.span.enter();
        event!(Level::INFO, "actor failed to start in time");

        Ok(())
    }

    pub fn get_mut(&mut self, index: Index) -> Result<&mut Node, BorrowError> {
        // Find the actor's node
        let node = self
            .nodes
            .get_mut(index)
            .ok_or(BorrowError::ActorNotFound)?;

        Ok(node)
    }

    pub fn remove(&mut self, index: Index) -> Result<Node, Error> {
        self.nodes.remove(index).context("actor doesn't exist")
    }

    /// Get the debug names of all active actors, except root.
    pub fn debug_names(&self) -> Vec<&'static str> {
        let mut debug_names = Vec::new();

        for (id, node) in &self.nodes {
            if id == self.root {
                continue;
            }

            debug_names.push(node.debug_name);
        }

        debug_names
    }
}

impl Default for ActorTree {
    fn default() -> Self {
        Self::new()
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
    #[error("failed to start actor, actor isn't pending to be started")]
    ActorNotPending,
    #[error("failed to start actor, error while returning to slot")]
    BorrowError(#[from] BorrowError),
}

fn debug_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    let before_generics = name.split('<').next().unwrap_or("Unknown");
    let after_modules = before_generics.split("::").last().unwrap_or("Unknown");
    after_modules
}
