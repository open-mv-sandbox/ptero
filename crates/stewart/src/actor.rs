use anyhow::Error;
use family::Family;

use crate::System;

/// Active message handler.
pub trait Actor {
    type Family: Family;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce(
        &mut self,
        message: <Self::Family as Family>::Member<'_>,
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
