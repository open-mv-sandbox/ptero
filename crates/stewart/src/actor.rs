use anyhow::Error;

use crate::System;

pub trait Actor {
    type Protocol: Protocol;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(
        &mut self,
        message: <Self::Protocol as Protocol>::Message<'a>,
    ) -> Result<AfterReduce, Error>;

    /// Process reduced messages.
    fn process(&mut self, system: &mut System) -> Result<(), Error>;
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum AfterReduce {
    Nothing,
    Process,
}

pub trait Protocol {
    type Message<'a>;
}
