use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use stewart::{ActorT, AddrT, After, Id, System};
use tracing::instrument;

/// Start actor that maps a value into another one.
#[instrument("map", skip_all)]
pub fn start_map<F, A, B>(
    system: &mut System,
    parent: Id,
    target: AddrT<B>,
    function: F,
) -> Result<AddrT<A>, Error>
where
    F: FnMut(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    let info = system.create_actor(parent)?;
    let actor = MapActor::<F, A, B> {
        function,
        target,
        _a: PhantomData,
    };
    system.start_actor(info, actor)?;

    Ok(info.addr())
}

struct MapActor<F, A, B> {
    function: F,
    target: AddrT<B>,
    _a: PhantomData<AtomicPtr<A>>,
}

impl<F, A, B> ActorT for MapActor<F, A, B>
where
    F: FnMut(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    type Message = A;

    fn handle(&mut self, system: &mut System, message: A) -> Result<After, Error> {
        // Immediately re-route the message
        let message = (self.function)(message);
        system.handle(self.target, message);
        Ok(After::Nothing)
    }
}
