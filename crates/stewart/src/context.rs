use std::any::Any;

use crate::{Address, Factory};

pub trait Context {
    /// Send a message on behalf of the actor to the target address.
    fn send_any(&self, address: usize, message: Box<dyn Any>);

    /// Start a new child actor on behalf of the actor.
    fn start_any(&self, factory: Box<dyn Factory>);
}

impl dyn '_ + Context {
    /// Send a message from this actor to a target address.
    pub fn send<T>(&self, address: Address<T>, message: T)
    where
        T: Any,
    {
        let message = Box::new(message);
        self.send_any(address.address, message);
    }

    /// Start a new child actor.
    pub fn start(&self, factory: impl Factory + 'static) {
        self.start_any(Box::new(factory));
    }
}
