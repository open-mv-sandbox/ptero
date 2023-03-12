use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use anyhow::Error;
use thiserror::Error;

use crate::{Id, System};

/// Shared thread-local processing schedule.
#[derive(Clone, Default)]
pub struct Schedule {
    queue: Rc<RefCell<VecDeque<Item>>>,
}

impl Schedule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an actor function to the schedule for processing.
    pub fn push(
        &mut self,
        id: Id,
        apply: fn(&mut System, Id) -> Result<(), Error>,
    ) -> Result<(), ScheduleError> {
        // TODO: Type safe? No requirement for manualy borrow-return?

        let mut queue = self.queue.try_borrow_mut().map_err(|_| ScheduleError)?;

        // If the queue already contains this actor, skip
        if queue.iter().any(|v| v.id == id) {
            return Ok(());
        }

        // Add the actor to the queue
        let item = Item { id, apply };
        queue.push_back(item);

        Ok(())
    }

    /// Run all queued actor processing tasks, until none remain.
    ///
    /// Running a process task may spawn new process tasks, so this is not guaranteed to ever
    /// return.
    pub fn run_until_idle(&self, system: &mut System) -> Result<(), Error> {
        system.cleanup_pending()?;

        while let Some(item) = self.take_next()? {
            // Apply process
            (item.apply)(system, item.id)?;

            system.cleanup_pending()?;
        }

        Ok(())
    }

    fn take_next(&self) -> Result<Option<Item>, ScheduleError> {
        let item = self
            .queue
            .try_borrow_mut()
            .map_err(|_| ScheduleError)?
            .pop_front();
        Ok(item)
    }
}

#[derive(Error, Debug)]
#[error("schedule is unavailable")]
pub struct ScheduleError;

struct Item {
    id: Id,
    apply: fn(&mut System, Id) -> Result<(), Error>,
}
