use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use thunderdome::Index;

use crate::{Actor, Sender};

/// Identifying handle of a created actor.
///
/// This can be used to manipulate the actor's data in an elevated way. Operations done using this
/// type are assumed to come from the actor itself. Sending this type around to other places would
/// likely be incorrect.
///
/// `Info` is intentionally !Send + !Sync. In most cases sending an addr between threads is a
/// mistake, as it's only valid for one `System`, and `System` is !Send + !Sync.
pub struct Info<A> {
    pub(crate) index: Index,
    _a: PhantomData<AtomicPtr<A>>,
}

impl<A> Info<A>
where
    A: Actor + 'static,
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

    pub fn sender(self) -> Sender<A::Family> {
        Sender::new::<A>(self.index)
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
