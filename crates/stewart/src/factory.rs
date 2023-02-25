use std::any::{type_name, Any};

use anyhow::Error;
use tracing::{event, Level};

use crate::{Actor, Context, Next};

/// Instructions for creating an actor on a runtime locally.
pub trait Factory {
    fn start(
        self: Box<Self>,
        ctx: &dyn Context,
        address: usize,
    ) -> Result<Box<dyn AnyActor>, Error>;
}

/// Downcasting interface for sending dynamic messages to actors.
pub trait AnyActor {
    fn handle_any(&mut self, ctx: &dyn Context, message: Box<dyn Any>) -> Result<Next, Error>;
}

impl<H> AnyActor for H
where
    H: Actor,
    H::Message: Any,
{
    fn handle_any(&mut self, ctx: &dyn Context, message: Box<dyn Any>) -> Result<Next, Error> {
        // TODO: Can we bypass AnyHandler's dynamic casting by redesigning the runtime to have type
        // specific channels? This might also eliminate the need for boxes.
        let result = message.downcast::<H::Message>();

        match result {
            Ok(message) => self.handle(ctx, *message),
            _ => {
                // This is an error with the caller, not the handler.
                // TODO: Report error to caller

                let handler_name = type_name::<H>();
                event!(
                    Level::ERROR,
                    handler = handler_name,
                    "failed to downcast message"
                );

                Ok(Next::Continue)
            }
        }
    }
}
