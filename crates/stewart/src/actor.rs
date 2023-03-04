use anyhow::Error;

use crate::{ActorAddr, System};

pub trait Actor {
    type Protocol: Protocol;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(
        &mut self,
        message: <Self::Protocol as Protocol>::Message<'a>,
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

pub trait Protocol {
    type Message<'a>;
}

/// Starting interface for actors.
pub trait Start: Actor + Sized {
    type Data;

    fn start(
        system: &mut System,
        addr: ActorAddr<<Self as Actor>::Protocol>,
        data: Self::Data,
    ) -> Result<Self, Error>;
}
