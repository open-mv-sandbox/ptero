use std::marker::PhantomData;

use thunderdome::Index;

/// Address for sending messages to an actor.
///
/// Addresses are intentionally !Send + !Sync. In most cases sending an addr between threads
/// is a mistake, as they're only valid for one `System`, and `System` is !Send + !Sync.
pub struct Addr<F> {
    index: Index,
    _p: PhantomData<*const F>,
}

impl<F> Addr<F> {
    pub(crate) fn from_index(index: Index) -> Self {
        Self {
            index,
            _p: PhantomData,
        }
    }

    pub(crate) fn index(&self) -> Index {
        self.index
    }
}

impl<F> Clone for Addr<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for Addr<F> {}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use crate::Addr;

    #[test]
    fn system_addr_option_same_size() {
        // This should be provided to us by the underlying Index type from thunderdome
        // But, it's good to verify just in case
        let size_plain = size_of::<Addr<()>>();
        let size_option = size_of::<Option<Addr<()>>>();
        assert_eq!(size_plain, size_option);
    }
}
