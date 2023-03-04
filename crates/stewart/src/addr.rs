use std::marker::PhantomData;

use thunderdome::Index;

use crate::Protocol;

pub struct ActorAddr<P> {
    id: Index,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const P>,
}

impl<P> ActorAddr<P> {
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

impl<M> Clone for ActorAddr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for ActorAddr<M> {}

// TODO: This can probably be auto-derived when the macro is smarter
impl<T> Protocol for ActorAddr<T> {
    type Message<'a> = Self;
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
