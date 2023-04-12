use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use anyhow::{Context as _, Error};
use stewart::{ActorId, Addr, State, System, SystemOptions, World};

use crate::Context;

/// Quick functional extension utilities for `Context`.
pub trait Functional {
    /// Create an actor that runs a function when receiving a message.
    ///
    /// If this callback returns false, the actor will be stopped.
    fn when<F, M>(&mut self, options: SystemOptions, function: F) -> Result<Addr<M>, Error>
    where
        F: FnMut(&mut World, ActorId, M) -> Result<bool, Error> + 'static,
        M: 'static;

    /// Create an actor that maps one message into another one and relays it.
    fn map<F, I, O>(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error>
    where
        F: FnMut(I) -> O + 'static,
        I: 'static,
        O: 'static;

    /// Same as `map`, but stops after handling one message.
    fn map_once<F, I, O>(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error>
    where
        F: FnOnce(I) -> O + 'static,
        I: 'static,
        O: 'static;
}

impl<'a> Functional for Context<'a> {
    fn when<F, M>(&mut self, options: SystemOptions, function: F) -> Result<Addr<M>, Error>
    where
        F: FnMut(&mut World, ActorId, M) -> Result<bool, Error> + 'static,
        M: 'static,
    {
        // In-line create a new system
        let system: WhenSystem<F, M> = WhenSystem { _w: PhantomData };
        let id = self.register(options, system);

        let (id, mut ctx) = self.create(id)?;
        let actor = When::<F, M> {
            function,
            _a: PhantomData,
        };
        ctx.start(id, actor)?;

        Ok(Addr::new(id))
    }

    fn map<F, I, O>(&mut self, target: Addr<O>, mut function: F) -> Result<Addr<I>, Error>
    where
        F: FnMut(I) -> O + 'static,
        I: 'static,
        O: 'static,
    {
        let addr = self.when(
            SystemOptions::high_priority(),
            move |world, _id, message| {
                let message = (function)(message);
                world.send(target, message);
                Ok(true)
            },
        )?;

        Ok(addr)
    }

    fn map_once<F, I, O>(&mut self, target: Addr<O>, function: F) -> Result<Addr<I>, Error>
    where
        F: FnOnce(I) -> O + 'static,
        I: 'static,
        O: 'static,
    {
        let mut function = Some(function);
        let addr = self.when(
            SystemOptions::high_priority(),
            move |world, _id, message| {
                let function = function
                    .take()
                    .context("map_once actor called more than once")?;
                let message = (function)(message);
                world.send(target, message);
                Ok(false)
            },
        )?;

        Ok(addr)
    }
}

struct WhenSystem<F, M> {
    _w: PhantomData<When<F, M>>,
}

impl<F, M> System for WhenSystem<F, M>
where
    F: FnMut(&mut World, ActorId, M) -> Result<bool, Error> + 'static,
    M: 'static,
{
    type Instance = When<F, M>;
    type Message = M;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((id, instance, message)) = state.next() {
            let result = (instance.function)(world, id, message)?;

            if !result {
                world.stop(id)?;
            }
        }

        Ok(())
    }
}

struct When<F, M> {
    function: F,
    _a: PhantomData<AtomicPtr<M>>,
}
