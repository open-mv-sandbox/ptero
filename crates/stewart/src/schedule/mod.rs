//! Actor processing scheduling.

mod schedule;

use anyhow::Error;

use crate::{After, System};

pub use self::schedule::{Schedule, ScheduleError};

pub trait Process {
    fn process(&mut self, system: &mut System) -> Result<After, Error>;
}
