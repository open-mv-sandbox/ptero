//! Helper types for storing and calling actors dynamically.

use std::{any::TypeId, ptr::NonNull};

use anyhow::Error;
use bevy_ptr::PtrMut;
use tracing::{event, Level};

use crate::{Actor, AfterProcess, AfterReduce, AnyActorAddr, System};

pub trait AnyActor {
    fn reduce(&mut self, message: AnyMessage) -> Result<AfterReduce, Error>;

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

impl<A> AnyActor for A
where
    A: Actor,
    A::Message<'static>: 'static,
{
    fn reduce(&mut self, message: AnyMessage) -> Result<AfterReduce, Error> {
        let message = match message.take::<A>() {
            Some(message) => message,
            None => {
                // This is not an error with the actor, but with the sending actor
                // TODO: Pass errors back
                event!(Level::ERROR, "incorrect dynamic message type");
                return Ok(AfterReduce::Nothing);
            }
        };

        Actor::reduce(self, message)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        Actor::process(self, system)
    }
}

pub struct AnyMessage<'a> {
    type_id: TypeId,
    slot_ptr: PtrMut<'a>,
}

impl<'a> AnyMessage<'a> {
    pub fn new<'b, A>(slot: &'a mut Option<A::Message<'b>>) -> Self
    where
        'b: 'a,
        A: AnyActorAddr,
    {
        let slot_ptr = NonNull::new(slot as *mut _ as *mut _).unwrap();
        let slot_ptr = unsafe { PtrMut::new(slot_ptr) };

        Self {
            type_id: TypeId::of::<A::Message<'static>>(),
            slot_ptr,
        }
    }

    pub fn take<A>(self) -> Option<A::Message<'a>>
    where
        A: Actor,
        A::Message<'static>: 'static,
    {
        // Make sure the protocol matches, which should give us a matching reference value
        if self.type_id != TypeId::of::<A::Message<'static>>() {
            return None;
        }

        // Very unsafe, very bad, downcast the message
        let slot = unsafe { self.slot_ptr.deref_mut::<Option<A::Message<'a>>>() };

        // Take the value out
        slot.take()
    }
}