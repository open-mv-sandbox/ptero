use std::any::Any;

// TODO: Can we remove 'static?
pub trait Actor {
    type Protocol: Protocol;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(&mut self, message: <Self::Protocol as Protocol>::Message<'a>) -> AfterReduce;

    /// Process reduced messages.
    fn process(&mut self);
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum AfterReduce {
    Nothing,
    Process,
}

pub trait Protocol: Any {
    type Message<'a>;
}
