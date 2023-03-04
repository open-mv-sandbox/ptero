use anyhow::Error;

use crate::{ActorAddr, ActorAddrF, StaticFamily, System};

pub trait Actor {
    type Message<'a>;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(&mut self, message: Self::Message<'a>) -> Result<AfterReduce, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum AfterReduce {
    Nothing,
    Process,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum AfterProcess {
    Nothing,
    Stop,
}

/// Starting interface for actors.
pub trait Start: Actor + Sized {
    type Data;

    fn start(
        system: &mut System,
        addr: ActorAddr<<Self as Actor>::Message<'static>>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}

/// Starting interface for actors with a custom family.
pub trait StartF: Actor + Sized {
    type Family;
    type Data;

    fn start(
        system: &mut System,
        addr: ActorAddrF<Self::Family>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}

impl<S> StartF for S
where
    S: Start,
{
    type Family = StaticFamily<Self::Message<'static>>;
    type Data = <Self as Start>::Data;

    fn start(
        system: &mut System,
        addr: ActorAddrF<Self::Family>,
        data: Self::Data,
    ) -> Result<Self, Error> {
        Start::start(system, addr, data)
    }
}
