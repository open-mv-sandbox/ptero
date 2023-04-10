use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use anyhow::Error;
use stewart::{Actor, Addr, After, Context, Id, Options, System};

/// Function-actor utility `Context` extension.
pub trait WhenExt<F, M> {
    /// Create an actor that runs a function when receiving a message.
    fn when(&mut self, function: F) -> Result<Addr<M>, Error>;
}

impl<'a, F, M> WhenExt<F, M> for Context<'a>
where
    F: FnMut(Context, M) -> Result<After, Error> + 'static,
    M: 'static,
{
    fn when(&mut self, function: F) -> Result<Addr<M>, Error> {
        let (id, mut ctx) = self.create()?;
        let actor = When::<F, M> {
            function,
            _a: PhantomData,
        };
        ctx.start(id, Options::default().with_high_priority(), actor)?;

        Ok(Addr::new(id))
    }
}

struct When<F, I> {
    function: F,
    _a: PhantomData<AtomicPtr<I>>,
}

impl<F, M> Actor for When<F, M>
where
    F: FnMut(Context, M) -> Result<After, Error> + 'static,
    M: 'static,
{
    type Message = M;

    fn handle(&mut self, system: &mut System, id: Id, message: M) -> Result<After, Error> {
        let ctx = Context::of(system, id);
        let after = (self.function)(ctx, message)?;
        Ok(after)
    }
}
