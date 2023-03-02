use std::marker::PhantomData;

use thunderdome::Index;

pub struct SystemAddr<P> {
    raw: RawSystemAddr,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const P>,
}

impl<P> SystemAddr<P> {
    pub fn from_raw(raw: RawSystemAddr) -> Self {
        Self {
            raw,
            _p: PhantomData,
        }
    }

    pub(crate) fn raw(&self) -> RawSystemAddr {
        self.raw
    }
}

impl<M> Clone for SystemAddr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for SystemAddr<M> {}

#[derive(Clone, Copy)]
pub struct RawSystemAddr(pub(crate) Index);

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::SystemAddr;

    #[test]
    fn system_addr_option_same_size() {
        // This should be provided to us by the underlying Index type from thunderdome
        // But, it's good to verify just in case
        let size_plain = size_of::<SystemAddr<()>>();
        let size_option = size_of::<Option<SystemAddr<()>>>();
        assert_eq!(size_plain, size_option);
    }
}
