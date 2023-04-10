use std::ops::{Deref, DerefMut};

use crate::{CreateError, Id, System};

/// Context bundle utility for system operations.
///
/// Very frequently, you want to perform operations 'within the scope of' certain information.
/// For example, creating an actor 'within the scope of' another actor.
/// Passing all this contextual information can quickly get very cumbersome, and adds a lot of
/// boilerplate visual noise.
///
/// This utility bundles all this contextual information for you.
pub struct Context<'a> {
    system: &'a mut System,
    current: Option<Id>,
}

impl<'a> Context<'a> {
    pub fn of(system: &'a mut System, id: Id) -> Self {
        Self {
            system,
            current: Some(id),
        }
    }

    pub fn root(system: &'a mut System) -> Self {
        Self {
            system,
            current: None,
        }
    }

    pub fn create(&mut self) -> Result<(Id, Context), CreateError> {
        let id = self.system.create(self.current)?;

        let ctx = Context {
            system: self.system,
            current: Some(id),
        };
        Ok((id, ctx))
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
