use std::marker::PhantomData;

use thunderdome::Index;

use crate::StaticFamily;

/// Address for sending messages to an actor, with a custom family.
pub struct ActorAddrF<F> {
    id: Index,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const F>,
}

impl<F> ActorAddrF<F> {
    pub(crate) fn from_id(id: Index) -> Self {
        Self {
            id,
            _p: PhantomData,
        }
    }

    pub(crate) fn id(&self) -> Index {
        self.id
    }
}

impl<F> Clone for ActorAddrF<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for ActorAddrF<F> {}

/// Address for sending messages to an actor.
pub type ActorAddr<T> = ActorAddrF<StaticFamily<T>>;

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::ActorAddrF;

    #[test]
    fn system_addr_option_same_size() {
        // This should be provided to us by the underlying Index type from thunderdome
        // But, it's good to verify just in case
        let size_plain = size_of::<ActorAddrF<()>>();
        let size_option = size_of::<Option<ActorAddrF<()>>>();
        assert_eq!(size_plain, size_option);
    }
}
