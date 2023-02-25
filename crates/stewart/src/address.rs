use std::{marker::PhantomData, sync::atomic::AtomicPtr};

/// Opaque target address of an actor.
pub struct Address<M> {
    address: usize,
    _m: PhantomData<AtomicPtr<M>>,
}

impl<M> Address<M> {
    pub fn from_raw(address: usize) -> Self {
        Self {
            address,
            _m: PhantomData,
        }
    }

    pub(crate) fn raw(&self) -> usize {
        self.address
    }
}

impl<M> Clone for Address<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for Address<M> {}
