use std::{any::TypeId, ffi::c_void, marker::PhantomData};

use tracing::{event, Level};

use crate::{Actor, AfterReduce, Protocol};

pub trait AnyActor {
    /// Take a message from a dynamic slot, and reduce it.
    ///
    /// # Safety
    /// - The given type ID **must** be correct for the 'static equivalent of the passed type.
    /// - The message **must** be valid and exclusively owned for the duration of the call.
    fn reduce(&mut self, message: AnyMessageSlot) -> AfterReduce;

    fn process(&mut self);
}

impl<A> AnyActor for A
where
    A: Actor,
{
    fn reduce(&mut self, mut message: AnyMessageSlot) -> AfterReduce {
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

pub struct AnyMessageSlot<'a> {
    protocol_id: TypeId,
    slot_ptr: *mut c_void,
    _lifetime: PhantomData<&'a mut u32>,
}

impl<'a> AnyMessageSlot<'a> {
    pub fn new<'b: 'a, P: Protocol>(slot: &'a mut Option<P::Message<'b>>) -> Self {
        Self {
            protocol_id: TypeId::of::<P>(),
            slot_ptr: slot as *mut _ as *mut _,
            _lifetime: PhantomData,
        }
    }

    pub fn take<'b, P: Protocol>(&'b mut self) -> Option<P::Message<'b>> {
        // Very unsafe, very bad, type check the message
        if self.protocol_id != TypeId::of::<P>() {
            return None;
        }

        // Very unsafe, very bad, downcast the message
        let typed_pointer = self.slot_ptr as *mut Option<P::Message<'b>>;
        let slot = unsafe { &mut *typed_pointer };

        // Take the value out
        slot.take()
    }
}
