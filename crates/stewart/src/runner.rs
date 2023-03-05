use thiserror::Error;
use tracing::{span, Level};

use crate::{
    system::{ActorEntry, DeferredAction},
    utils::VoidActor,
    System,
};

/// System running utility.
pub struct Runner {
    /// Dummy placeholder, keep one around to avoid re-allocating.
    dummy_entry: Option<ActorEntry>,
}

impl Runner {
    pub fn new() -> Self {
        let dummy_entry = ActorEntry {
            span: span!(Level::ERROR, "unreachable"),
            actor: Box::new(VoidActor),
            queued: false,
        };

        Self {
            dummy_entry: Some(dummy_entry),
        }
    }

    pub fn run_until_idle(&mut self, system: &mut System) -> Result<(), RunnerError> {
        loop {
            self.run_deferred(system)?;

            if let Some(index) = system.queue_next() {
                system.process_at(index, &mut self.dummy_entry)?;
            } else {
                break;
            };
        }

        Ok(())
    }

    fn run_deferred(&mut self, system: &mut System) -> Result<(), RunnerError> {
        while let Some(action) = system.deferred_next() {
            match action {
                DeferredAction::Start(factory) => {
                    system.start_immediate(factory, &mut self.dummy_entry)?
                }
            }
        }

        Ok(())
    }
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("unknown internal error, this is a bug in stewart")]
    Internal(#[from] anyhow::Error),
}
