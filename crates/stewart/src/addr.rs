use std::marker::PhantomData;

use thunderdome::Index;

pub struct SystemAddr<M> {
    any: AnySystemAddr,
    /// Intentionally !Send + !Sync
    _m: PhantomData<*const M>,
}

impl<M> SystemAddr<M> {
    pub fn from_any(any: AnySystemAddr) -> Self {
        Self {
            any,
            _m: PhantomData,
        }
    }

    pub(crate) fn any(&self) -> AnySystemAddr {
        self.any
    }
}

impl<M> Clone for SystemAddr<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M> Copy for SystemAddr<M> {}

#[derive(Clone, Copy)]
pub struct AnySystemAddr(pub(crate) Index);

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::SystemAddr;

    #[test]
    fn system_addr_is_nonzero() {
        let size_plain = size_of::<SystemAddr<()>>();
        let size_option = size_of::<Option<SystemAddr<()>>>();
        assert_eq!(size_plain, size_option);
    }
}
