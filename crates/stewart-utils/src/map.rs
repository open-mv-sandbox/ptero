use std::marker::PhantomData;
use std::sync::atomic::AtomicPtr;

use anyhow::Error;
use stewart::{Actor, Addr, After, Context, Options};
use tracing::instrument;

/// Mapping utility `Context` extension.
pub trait MapExt<F, I, O> {
    /// Create an actor that maps one message into another one and relays it.
    fn map(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error>;

    /// Same as `map`, but stops after handling one message.
    fn map_once(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error>;
}

impl<'a, F, I, O> MapExt<F, I, O> for Context<'a>
where
    F: FnMut(I) -> O + 'static,
    I: 'static,
    O: 'static,
{
    #[instrument("map", skip_all)]
    fn map(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error> {
        let mut ctx = self.create()?;
        let actor = Map::<F, I, O> {
            function,
            target,
            _a: PhantomData,
        };
        ctx.start(Options::default().with_high_priority(), actor)?;

        Ok(ctx.addr()?)
    }

    #[instrument("map-once", skip_all)]
    fn map_once(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error> {
        let mut ctx = self.create()?;
        let actor = MapOnce::<F, I, O> {
            function,
            target,
            _a: PhantomData,
        };
        ctx.start(Options::default().with_high_priority(), actor)?;

        Ok(ctx.addr()?)
    }
}

struct Map<F, I, O> {
    function: F,
    target: Addr<O>,
    _a: PhantomData<AtomicPtr<I>>,
}

impl<F, I, O> Actor for Map<F, I, O>
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

struct MapOnce<F, I, O> {
    function: F,
    target: Addr<O>,
    _a: PhantomData<AtomicPtr<I>>,
}

impl<F, I, O> Actor for MapOnce<F, I, O>
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
        Ok(After::Stop)
    }
}
