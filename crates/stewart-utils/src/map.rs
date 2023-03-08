use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use stewart::{ActorT, AddrT, AfterProcess, AfterReduce, Id, System};
use tracing::instrument;

/// Start actor that maps a value into another one.
#[instrument("map", skip_all)]
pub fn start_map<F, A, B>(
    system: &mut System,
    parent: Id,
    map: F,
    target: AddrT<B>,
) -> Result<AddrT<A>, Error>
where
    F: FnMut(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    // TODO: No longer needs the static requirement

    let info = system.create_actor(parent)?;
    let actor = MapActor {
        map,
        target,
        _a: PhantomData,
    };
    system.start_actor(info, actor)?;

    Ok(info.addr())
}

struct MapActor<F, A, B> {
    map: F,
    target: AddrT<B>,
    _a: PhantomData<AtomicPtr<A>>,
}

impl<F, A, B> ActorT for MapActor<F, A, B>
where
    F: FnMut(A) -> B,
    A: 'static,
    B: 'static,
{
    type Message = A;

    fn reduce(&mut self, system: &mut System, message: A) -> Result<AfterReduce, Error> {
        // Immediately re-route the message
        let message = (self.map)(message);
        system.handle(self.target, message);
        Ok(AfterReduce::Nothing)
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        Ok(AfterProcess::Nothing)
    }
}
