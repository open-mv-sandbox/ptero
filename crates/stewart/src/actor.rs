use better_any::TidAble;

pub trait Actor {
    type Message<'a>: TidAble<'a>;

    /// Handle a message in-place, storing it as appropriate until processing.
    fn reduce<'a>(&mut self, message: Self::Message<'a>) -> AfterReduce;

    /// Process reduced messages.
    fn process(&mut self);
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum AfterReduce {
    Nothing,
    Process,
}
