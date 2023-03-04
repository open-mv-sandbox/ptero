use anyhow::Error;

use crate::{ActorAddrF, Family, StaticFamily, System};

pub trait Actor {
    type Message: 'static;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce(&mut self, message: Self::Message) -> Result<AfterReduce, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

pub trait ActorF {
    type Family: Family;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(
        &mut self,
        message: <Self::Family as Family>::Member<'a>,
    ) -> Result<AfterReduce, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

impl<A> ActorF for A
where
    A: Actor,
{
    type Family = StaticFamily<<Self as Actor>::Message>;

    fn reduce<'a>(
        &mut self,
        message: <Self::Family as Family>::Member<'a>,
    ) -> Result<AfterReduce, Error> {
        self.reduce(message)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        self.process(system)
    }
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
pub trait Start: ActorF + Sized {
    type Data;

    fn start(
        system: &mut System,
        addr: ActorAddrF<<Self as ActorF>::Family>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}
