use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use anyhow::Error;
use stewart::{Actor, Addr, After, Context, Options};

/// Function-actor utility `Context` extension.
pub trait WhenExt<F, I> {
    /// Create an actor that runs a function when receiving a message.
    fn when(&mut self, function: F) -> Result<Addr<I>, Error>;
}

impl<'a, F, I> WhenExt<F, I> for Context<'a>
where
    F: FnMut(&mut Context, I) -> Result<After, Error> + 'static,
    I: 'static,
{
    fn when(&mut self, function: F) -> Result<Addr<I>, Error> {
        let mut ctx = self.create()?;
        let actor = When::<F, I> {
            function,
            _a: PhantomData,
        };
        ctx.start(Options::default().with_high_priority(), actor)?;

        Ok(ctx.addr()?)
    }
}

struct When<F, I> {
    function: F,
    _a: PhantomData<AtomicPtr<I>>,
}

impl<F, I> Actor for When<F, I>
where
    F: FnMut(&mut Context, I) -> Result<After, Error> + 'static,
    I: 'static,
{
    type Message = I;

    fn handle(&mut self, ctx: &mut Context, message: I) -> Result<After, Error> {
        let after = (self.function)(ctx, message)?;
        Ok(after)
    }
}
