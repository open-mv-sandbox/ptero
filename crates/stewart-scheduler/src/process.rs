use anyhow::Error;
use stewart::{handler::After, System};

pub trait Process {
    fn process(&mut self, system: &mut System) -> Result<After, Error>;
}
