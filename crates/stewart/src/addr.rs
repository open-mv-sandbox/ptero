use std::marker::PhantomData;

use thunderdome::Index;

pub struct ActorAddr<P> {
    raw: RawAddr,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const P>,
}

impl<P> ActorAddr<P> {
    pub fn from_raw(raw: RawAddr) -> Self {
        Self {
            raw,
            _p: PhantomData,
        }
    }

    pub(crate) fn raw(&self) -> RawAddr {
        self.raw
    }
}

impl<M> Clone for ActorAddr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for ActorAddr<M> {}

#[derive(Clone, Copy)]
pub struct RawAddr(pub(crate) Index);

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
