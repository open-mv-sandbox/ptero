use anyhow::Error;
use stewart::{Addr, After, Context};

use crate::WhenExt;

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
    fn map(&mut self, target: Addr<O>, mut function: F) -> Result<Addr<I>, Error> {
        let addr = self.when(move |mut ctx, message| {
            let message = (function)(message);
            ctx.send(target, message);
            Ok(After::Continue)
        })?;

        Ok(addr)
    }

    fn map_once(&mut self, target: Addr<O>, mut function: F) -> Result<Addr<I>, Error> {
        let addr = self.when(move |mut ctx, message| {
            let message = (function)(message);
            ctx.send(target, message);
            Ok(After::Stop)
        })?;

        Ok(addr)
    }
}
