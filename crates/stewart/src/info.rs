use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use thunderdome::Index;

use crate::Actor;

/// Identifying handle of a created actor.
///
/// This can be used to manipulate the actor's data in an elevated way. Operations done using this
/// type are assumed to come from the actor itself. Sending this type around to other places would
/// likely be incorrect.
///
/// `Info` is intentionally !Send + !Sync. In most cases sending an addr between threads is a
/// mistake, as it's only valid for one `System`, and `System` is !Send + !Sync.
pub struct Info<A> {
    index: Index,
    _a: PhantomData<AtomicPtr<A>>,
}

impl<A: Actor> Info<A> {
    pub(crate) fn new(index: Index) -> Self {
        Self {
            index,
            _a: PhantomData,
        }
    }

    pub(crate) fn index(&self) -> Index {
        self.index
    }

    pub fn id(self) -> Id {
        Id(Some(self.index))
    }

    pub fn addr(self) -> Addr<A::Family> {
        Addr {
            index: self.index,
            _p: PhantomData,
        }
    }
}

impl<A> Clone for Info<A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A> Copy for Info<A> {}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Id(pub(crate) Option<Index>);

impl Id {
    /// Address ID of the root of the system.
    ///
    /// You can use this to start actors with no parent other than the system root. These actors
    /// live until explicitly stopped, or until the system is dropped. This is useful if you don't
    /// have any actors yet, or if you want to start an actor that isn't a child of any other
    /// actor and thus lives longer.
    pub fn root() -> Self {
        Self(None)
    }
}

/// Address for sending messages to an actor.
///
/// `Addr` is intentionally !Send + !Sync. In most cases sending an addr between threads is a
/// mistake, as it's only valid for one `System`, and `System` is !Send + !Sync.
pub struct Addr<F> {
    index: Index,
    _p: PhantomData<*const F>,
}

impl<F> Addr<F> {
    pub(crate) fn index(self) -> Index {
        self.index
    }
}

impl<F> Clone for Addr<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for Addr<F> {}
