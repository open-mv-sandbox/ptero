use anyhow::Error;

use crate::Context;

/// Actor message handling trait.
pub trait Actor {
    type Message;

    fn handle(&mut self, ctx: &dyn Context, message: Self::Message) -> Result<Next, Error>;
}

/// What should be done with the actor after returning from the message handler.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Next {
    Continue,
    Stop,
}
