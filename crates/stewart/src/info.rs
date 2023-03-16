use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use thunderdome::Index;

use crate::Actor;

/// Identifying handle of a created actor.
///
/// This can be used to manipulate the actor's data in an elevated way. Operations done using this
/// type are assumed to come from the actor itself. Sending this type around to other places would
/// likely be incorrect.
pub struct Info<A> {
    pub(crate) index: Index,
    _a: PhantomData<AtomicPtr<A>>,
}

impl<A> Info<A>
where
    A: Actor,
{
    pub(crate) fn new(index: Index) -> Self {
        Self {
            index,
            _a: PhantomData,
        }
    }

    pub fn id(self) -> Id {
        Id { index: self.index }
    }

    pub fn addr(self) -> Addr<A::Message> {
        Addr::new(self.index)
    }
}

impl<A> Clone for Info<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Info<A> {}

/// Untyped identifier of an actor.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Id {
    pub(crate) index: Index,
}

/// Identifier of an actor, with its associated message type.
pub struct Addr<M> {
    pub(crate) index: Index,
    _m: PhantomData<AtomicPtr<M>>,
}

impl<M> Addr<M> {
    pub(crate) fn new(index: Index) -> Self {
        Addr {
            index,
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
