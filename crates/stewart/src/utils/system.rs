use anyhow::Error;

use crate::{system::StartError, Actor, Addr, System};

/// Helpers for additional common patterns, that only use public functions.
pub trait SystemExt {
    /// Start actor where the third argument of the start function is the parameters of the actor.
    fn start_with<T, F, A>(
        &mut self,
        id: &'static str,
        data: T,
        start: F,
    ) -> Result<Addr<A::Family>, StartError>
    where
        F: FnOnce(&mut System, Addr<A::Family>, T) -> Result<A, Error>,
        A: Actor + 'static;
}

impl SystemExt for System {
    fn start_with<T, F, A>(
        &mut self,
        id: &'static str,
        data: T,
        start: F,
    ) -> Result<Addr<A::Family>, StartError>
    where
        F: FnOnce(&mut System, Addr<A::Family>, T) -> Result<A, Error>,
        A: Actor + 'static,
    {
        self.start(id, move |s, a| (start)(s, a, data))
    }
}
