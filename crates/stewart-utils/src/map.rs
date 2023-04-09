use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use stewart::{Actor, Addr, After, Context, Options};
use tracing::instrument;

/// Start actor that maps a value into another one.
#[instrument("map", skip_all)]
pub fn start_map<F, I, O>(ctx: &mut Context, target: Addr<O>, function: F) -> Result<Addr<I>, Error>
where
    F: FnMut(I) -> O + 'static,
    I: 'static,
    O: 'static,
{
    let mut ctx = ctx.create()?;
    let actor = MapActor::<F, I, O> {
        function,
        target,
        _a: PhantomData,
    };
    ctx.start(Options::default().with_high_priority(), actor)?;

    Ok(ctx.addr()?)
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

    fn handle(&mut self, ctx: &mut Context, message: I) -> Result<After, Error> {
        // Immediately re-route the message
        let message = (self.function)(message);
        ctx.send(self.target, message);
        Ok(After::Continue)
    }
}
