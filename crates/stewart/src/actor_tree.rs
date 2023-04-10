use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};
use tracing::{event, Level, Span};

use crate::{node::Node, slot::ActorSlot, Actor, Options};

#[derive(Default)]
pub struct ActorTree {
    nodes: Arena<Node>,
    pending_start: Vec<Id>,
}

impl ActorTree {
    pub fn create(&mut self, parent: Option<Id>) -> Result<Id, CreateError> {
        // Continual span is inherited from the create addr callsite
        let span = Span::current();

        // Link to the parent
        if let Some(parent) = parent {
            self.nodes
                .get_mut(parent.index)
                .ok_or(CreateError::ParentDoesNotExist)?;
        }

        // Create the node
        let node = Node::new(span, parent);
        let index = self.nodes.insert(node);
        let id = Id { index };

        // Track the address so we can clean it up if it doesn't get started in time
        self.pending_start.push(id);

        Ok(id)
    }

    pub fn start<A>(&mut self, id: Id, options: Options, actor: A) -> Result<(), StartError>
    where
        A: Actor,
    {
        // Check if it's pending, and remove it
        let pending_index = self
            .pending_start
            .iter()
            .position(|i| *i == id)
            .ok_or(StartError::ActorAlreadyStarted)?;
        self.pending_start.remove(pending_index);

        // Box the actor, so we can access it dynamically
        let slot = ActorSlot {
            bin: Vec::new(),
            actor,
        };

        // Apply the start
        let node = self.get_mut(id).context("failed to get node")?;
        node.set_debug_name(debug_name::<A>());
        node.set_options(options);
        node.store(Box::new(slot))
            .context("failed to store actor")?;

        Ok(())
    }

    /// Clean up actors that didn't start in time, and thus failed.
    pub fn cleanup_pending(&mut self) -> Result<(), Error> {
        // Intentionally in reverse order, clean up children before parents
        while let Some(id) = self.pending_start.pop() {
            self.cleanup_pending_at(id)?;
        }

        Ok(())
    }

    fn cleanup_pending_at(&mut self, id: Id) -> Result<(), Error> {
        let node = self.remove(id)?;

        let span = node.span();
        let _enter = span.enter();
        event!(Level::INFO, "actor failed to start in time");

        Ok(())
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut Node> {
        self.nodes.get_mut(id.index)
    }

    pub fn remove(&mut self, id: Id) -> Result<Node, Error> {
        // Remove all children
        // TODO: Optimize hierarchy walking
        let children: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.parent() == Some(id))
            .map(|(index, _)| Id { index })
            .collect();
        for child in children {
            self.remove(child)?;
        }

        // Remove the given actor itself
        let node = self
            .nodes
            .remove(id.index)
            .context("tried to remove actor that doesn't exist")?;

        Ok(node)
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

/// Untyped identifier of an actor.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Id {
    index: Index,
}

fn debug_name<T>() -> &'static str {
    let name = std::any::type_name::<T>();
    let before_generics = name.split('<').next().unwrap_or("Unknown");
    let after_modules = before_generics.split("::").last().unwrap_or("Unknown");
    after_modules
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CreateError {
    #[error("actor isn't pending to be started")]
    ParentDoesNotExist,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum StartError {
    #[error("actor already started")]
    ActorAlreadyStarted,
    #[error("internal error")]
    Internal(#[from] Error),
}
