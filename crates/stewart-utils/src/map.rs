use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use family::{utils::FamilyT, Family};
use stewart::{Actor, Addr, AddrT, AfterProcess, AfterReduce, Id, System};
use tracing::instrument;

/// Start actor that maps a value into another one.
#[instrument("map", skip_all)]
pub fn start_map<F, A, B>(
    system: &mut System,
    parent: Id,
    function: F,
    target: AddrT<B>,
) -> Result<Addr<A>, Error>
where
    F: FnMut(A::Member<'_>) -> B + 'static,
    A: Family,
    B: 'static,
{
    let info = system.create_actor(parent)?;
    let actor = MapActor {
        function,
        target,
        _a: PhantomData,
    };
    system.start_actor(info, actor)?;

    Ok(info.addr())
}

pub fn start_map_t<F, A, B>(
    system: &mut System,
    parent: Id,
    mut function: F,
    target: AddrT<B>,
) -> Result<AddrT<A>, Error>
where
    F: FnMut(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    let function = move |a: <FamilyT<A> as Family>::Member<'_>| function(a.0);
    start_map::<_, FamilyT<A>, _>(system, parent, function, target)
}

struct MapActor<F, A, B> {
    function: F,
    target: AddrT<B>,
    _a: PhantomData<AtomicPtr<A>>,
}

impl<F, A, B> Actor for MapActor<F, A, B>
where
    F: FnMut(A::Member<'_>) -> B + 'static,
    A: Family,
    B: 'static,
{
    type Family = A;

    fn reduce(
        &mut self,
        system: &mut System,
        message: A::Member<'_>,
    ) -> Result<AfterReduce, Error> {
        // Immediately re-route the message
        let message = (self.function)(message);
        system.handle(self.target, message);
        Ok(AfterReduce::Nothing)
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        Ok(AfterProcess::Nothing)
    }
}
