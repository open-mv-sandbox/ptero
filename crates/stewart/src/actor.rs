use anyhow::Error;

use crate::{family::Family, ActorAddr, System};

/// Active message handler.
pub trait Actor {
    type Family: Family;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(
        &mut self,
        message: <Self::Family as Family>::Member<'a>,
    ) -> Result<AfterReduce, Error>;

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
        addr: ActorAddr<<Self as Actor>::Family>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}
