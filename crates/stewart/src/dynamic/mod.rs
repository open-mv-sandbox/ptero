//! Helper types for storing and calling actors dynamically.

use std::{any::TypeId, ptr::NonNull};

use bevy_ptr::PtrMut;
use tracing::{event, Level};

use crate::{Actor, AfterReduce, Protocol};

pub trait AnyActor {
    fn reduce(&mut self, message: AnyMessage) -> AfterReduce;

    fn process(&mut self);
}

impl<A> AnyActor for A
where
    A: Actor,
    A::Protocol: 'static,
{
    fn reduce(&mut self, message: AnyMessage) -> AfterReduce {
        let message = match message.take::<A::Protocol>() {
            Some(message) => message,
            None => {
                event!(Level::ERROR, "incorrect dynamic message type");
                return AfterReduce::Nothing;
            }
        };

        Actor::reduce(self, message)
    }

    fn process(&mut self) {
        Actor::process(self);
    }
}

pub struct AnyMessage<'a> {
    protocol_id: TypeId,
    slot_ptr: PtrMut<'a>,
}

impl<'a> AnyMessage<'a> {
    pub fn new<'b: 'a, P: Protocol + 'static>(slot: &'a mut Option<P::Message<'b>>) -> Self {
        let slot_ptr = NonNull::new(slot as *mut _ as *mut _).unwrap();
        let slot_ptr = unsafe { PtrMut::new(slot_ptr) };

        Self {
            protocol_id: TypeId::of::<P>(),
            slot_ptr,
        }
    }

    pub fn take<P: Protocol + 'static>(self) -> Option<P::Message<'a>> {
        // Make sure the protocol matches, which should give us a matching reference value
        if self.protocol_id != TypeId::of::<P>() {
            return None;
        }

        // Very unsafe, very bad, downcast the message
        let slot = unsafe { self.slot_ptr.deref_mut::<Option<P::Message<'a>>>() };

        // Take the value out
        slot.take()
    }
}
