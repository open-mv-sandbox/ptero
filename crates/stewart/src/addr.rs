use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use thunderdome::Index;

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
