use std::ops::{Deref, DerefMut};

use stewart::{ActorId, CreateError, World};

/// Context bundle utility for system operations.
///
/// Very frequently, you want to perform operations 'within the scope of' certain information.
/// For example, creating an actor 'within the scope of' another actor.
/// Passing all this contextual information can quickly get very cumbersome, and adds a lot of
/// boilerplate visual noise.
///
/// This utility bundles all this contextual information for you.
pub struct Context<'a> {
    world: &'a mut World,
    current: Option<ActorId>,
}

impl<'a> Context<'a> {
    pub fn of(world: &'a mut World, id: ActorId) -> Self {
        Self {
            world,
            current: Some(id),
        }
    }

    pub fn root(world: &'a mut World) -> Self {
        Self {
            world,
            current: None,
        }
    }

    pub fn create(&mut self) -> Result<(ActorId, Context), CreateError> {
        let id = self.world.create(self.current)?;

        let ctx = Context {
            world: self.world,
            current: Some(id),
        };
        Ok((id, ctx))
    }
}

impl<'a> Deref for Context<'a> {
    type Target = World;

    fn deref(&self) -> &World {
        &self.world
    }
}

impl<'a> DerefMut for Context<'a> {
    fn deref_mut(&mut self) -> &mut World {
        &mut self.world
    }
}
