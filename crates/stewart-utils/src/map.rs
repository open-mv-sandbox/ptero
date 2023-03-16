use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use stewart::{Actor, Addr, After, Id, Options, System};
use tracing::instrument;

/// Start actor that maps a value into another one.
#[instrument("map", skip_all)]
pub fn start_map<F, I, O>(
    system: &mut System,
    parent: Id,
    target: Addr<O>,
    function: F,
) -> Result<Addr<I>, Error>
where
    F: FnMut(I) -> O + 'static,
    I: 'static,
    O: 'static,
{
    let info = system.create(parent)?;
    let actor = MapActor::<F, I, O> {
        function,
        target,
        _a: PhantomData,
    };
    system.start(info, Options::default().high_priority(), actor)?;

    Ok(info.addr())
}

struct MapActor<F, I, O> {
    function: F,
    target: Addr<O>,
    _a: PhantomData<AtomicPtr<I>>,
}

impl<F, I, O> Actor for MapActor<F, I, O>
where
    F: FnMut(I) -> O + 'static,
    I: 'static,
    O: 'static,
{
    type Message = I;

    fn handle(&mut self, system: &mut System, message: I) -> Result<After, Error> {
        // Immediately re-route the message
        let message = (self.function)(message);
        system.send(self.target, message);
        Ok(After::Nothing)
    }
}
