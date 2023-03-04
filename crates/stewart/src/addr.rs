use std::marker::PhantomData;

use thunderdome::Index;

use crate::Family;

pub struct ActorAddr<F> {
    id: Index,
    /// Intentionally !Send + !Sync
    _p: PhantomData<*const F>,
}

impl<F> ActorAddr<F> {
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

impl<F> Clone for ActorAddr<F> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for ActorAddr<F> {}

// TODO: Make derive macro smarter so we don't have to manually derive on generics
impl<F> Family for ActorAddr<F> {
    type Member<'a> = Self;
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
