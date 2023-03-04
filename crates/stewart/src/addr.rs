use std::marker::PhantomData;

use thunderdome::Index;

use crate::Family;

pub trait AnyActorAddr: 'static {
    type Message<'a>;

    /// For internal use only.
    fn from_id(id: Index) -> Self;

    /// For internal use only.
    fn id(&self) -> Index;
}

pub struct ActorAddr<M> {
    id: Index,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const M>,
}

impl<M> Clone for ActorAddr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for ActorAddr<M> {}

impl<M: 'static> AnyActorAddr for ActorAddr<M> {
    type Message<'a> = M;

    fn from_id(id: Index) -> Self {
        Self {
            id,
            _p: PhantomData,
        }
    }

    fn id(&self) -> Index {
        self.id
    }
}

/// Family variant of address.
pub struct ActorAddrF<F> {
    id: Index,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const F>,
}

impl<F> Clone for ActorAddrF<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for ActorAddrF<F> {}

impl<F: Family + 'static> AnyActorAddr for ActorAddrF<F> {
    type Message<'a> = F::Member<'a>;

    fn from_id(id: Index) -> Self {
        Self {
            id,
            _p: PhantomData,
        }
    }

    fn id(&self) -> Index {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::ActorAddr;

    #[test]
    fn system_addr_option_same_size() {
        // This should be provided to us by the underlying Index type from thunderdome
        // But, it's good to verify just in case
        let size_plain = size_of::<ActorAddr<()>>();
        let size_option = size_of::<Option<ActorAddr<()>>>();
        assert_eq!(size_plain, size_option);
    }
}
