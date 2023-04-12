use std::collections::HashMap;

use anyhow::{Context, Error};
use thiserror::Error;
use thunderdome::{Arena, Index};

use crate::SystemId;

#[derive(Default)]
pub struct Tree {
    nodes: Arena<Node>,
}

impl Tree {
    pub fn insert(&mut self, node: Node) -> Result<Index, CreateError> {
        // Link to the parent
        if let Some(parent) = node.parent {
            self.nodes
                .get_mut(parent)
                .ok_or(CreateError::ParentNotFound)?;
        }

        // Insert the node
        let index = self.nodes.insert(node);

        Ok(index)
    }

    pub fn get(&self, index: Index) -> Option<&Node> {
        self.nodes.get(index)
    }

    pub fn get_mut(&mut self, index: Index) -> Option<&mut Node> {
        self.nodes.get_mut(index)
    }

    pub fn remove<F>(&mut self, index: Index, mut on_removed: F) -> Result<(), Error>
    where
        F: FnMut(Node),
    {
        self.remove_inner(index, &mut on_removed)
    }

    fn remove_inner<F>(&mut self, index: Index, on_removed: &mut F) -> Result<(), Error>
    where
        F: FnMut(Node),
    {
        // TODO: Optimize hierarchy walking

        // Remove all children, always first recursively on purpose
        let children: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, n)| n.parent() == Some(index))
            .map(|(index, _)| index)
            .collect();
        for child in children {
            self.remove_inner(child, on_removed)?;
        }

        // Remove the given actor itself
        let node = self
            .nodes
            .remove(index)
            .context("tried to remove actor that doesn't exist")?;
        on_removed(node);

        Ok(())
    }

    pub fn count(&self) -> HashMap<SystemId, usize> {
        let mut counts = HashMap::new();

        for (_, node) in &self.nodes {
            let entry = counts.entry(node.system).or_default();
            *entry = *entry + 1;
        }

        counts
    }
}

pub struct Node {
    system: SystemId,
    parent: Option<Index>,
}

impl Node {
    pub fn new(system: SystemId, parent: Option<Index>) -> Self {
        Self { system, parent }
    }

    pub fn system(&self) -> SystemId {
        self.system
    }

    pub fn parent(&self) -> Option<Index> {
        self.parent
    }
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum CreateError {
    #[error("parent not found")]
    ParentNotFound,
}
