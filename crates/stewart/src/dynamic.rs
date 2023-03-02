use better_any::{Tid, TidExt};
use tracing::{event, Level};

use crate::{Actor, AfterReduce};

pub trait AnyActor {
    /// TODO: Can we eliminate the need for a Box here?
    fn reduce<'a>(&mut self, message: Box<dyn Tid<'a> + 'a>) -> AfterReduce;
    fn process(&mut self);
}

impl<A> AnyActor for A
where
    A: Actor,
{
    fn reduce<'a>(&mut self, message: Box<dyn Tid<'a> + 'a>) -> AfterReduce {
        // Downcast the message
        let result = message.downcast_box().ok();
        let message: Box<_> = match result {
            Some(message) => message,
            None => {
                event!(Level::ERROR, "failed to downcast dynamic message");
                return AfterReduce::Nothing;
            }
        };

        // Perform reducing
        Actor::reduce(self, *message)
    }

    fn process(&mut self) {
        Actor::process(self);
    }
}
