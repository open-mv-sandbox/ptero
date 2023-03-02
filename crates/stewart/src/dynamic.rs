use better_any::{Tid, TidExt};
use tracing::{event, Level};

use crate::{Actor, AfterReduce};

pub trait AnyActor {
    fn reduce<'a>(&mut self, message: &mut (dyn Tid<'a> + 'a)) -> AfterReduce;
    fn process(&mut self);
}

impl<A> AnyActor for A
where
    A: Actor,
{
    fn reduce<'a>(&mut self, message: &mut (dyn Tid<'a> + 'a)) -> AfterReduce {
        // Downcast the message
        let result = message.downcast_mut();
        let message_option: &mut Option<_> = match result {
            Some(message_option) => message_option,
            None => {
                event!(Level::ERROR, "failed to downcast dynamic message");
                return AfterReduce::Nothing;
            }
        };

        // Perform reducing
        let message = message_option.take().unwrap();
        Actor::reduce(self, message)
    }

    fn process(&mut self) {
        Actor::process(self);
    }
}
