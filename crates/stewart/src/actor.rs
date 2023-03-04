use anyhow::Error;

use crate::{ActorAddr, ActorAddrF, Family, System};

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

pub trait StartF: Actor + Sized {
    type Family: Family<Member<'static> = <Self as Actor>::Message<'static>>;
    type Data;

    fn start(
        system: &mut System,
        addr: ActorAddrF<Self::Family>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}
