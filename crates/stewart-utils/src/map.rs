use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use family::{
    utils::{FamilyT, MemberT},
    Family,
};
use stewart::{Actor, Addr, AddrT, AfterProcess, AfterReduce, Id, System};
use tracing::instrument;

/// Start actor that maps a value into another one.
#[instrument("map", skip_all)]
pub fn start_map<'a, F, A, B, C>(
    system: &mut System,
    parent: Id,
    function: F,
    target: Addr<B>,
) -> Result<Addr<A>, Error>
where
    F: FnMut(A::Member<'_>) -> C + 'static,
    A: Family,
    B: Family,
    for<'b> C: Into<B::Member<'a>> + 'b,
{
    let info = system.create_actor(parent)?;
    let actor = MapActor::<F, A, B, C> {
        function,
        target,
        _a: PhantomData,
        _c: PhantomData,
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
    let function = move |a: <FamilyT<A> as Family>::Member<'_>| MemberT(function(a.0));
    start_map::<_, FamilyT<A>, _, _>(system, parent, function, target)
}

struct MapActor<F, A, B, C> {
    function: F,
    target: Addr<B>,
    _a: PhantomData<AtomicPtr<A>>,
    _c: PhantomData<AtomicPtr<C>>,
}

impl<'a, F, A, B, C> Actor for MapActor<F, A, B, C>
where
    F: FnMut(A::Member<'_>) -> C + 'static,
    A: Family,
    B: Family,
    C: Into<B::Member<'a>>,
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
