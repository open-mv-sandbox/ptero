//! Helper types for storing and calling actors dynamically.

use std::{any::TypeId, ptr::NonNull};

use anyhow::Error;
use bevy_ptr::PtrMut;
use tracing::{event, Level};

use crate::{family::Family, Actor, AfterProcess, AfterReduce, System};

pub trait AnyActor {
    fn reduce(&mut self, message: AnyMessage) -> Result<AfterReduce, Error>;

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error>;
}

impl<A> AnyActor for A
where
    A: Actor,
{
    fn reduce(&mut self, message: AnyMessage) -> Result<AfterReduce, Error> {
        let message = match message.take::<A::Family>() {
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
    family_id: TypeId,
    slot_ptr: PtrMut<'a>,
}

impl<'a> AnyMessage<'a> {
    pub fn new<'b, F>(slot: &'a mut Option<F::Member<'b>>) -> Self
    where
        'b: 'a,
        F: Family,
    {
        let slot_ptr = NonNull::new(slot as *mut _ as *mut _).unwrap();
        let slot_ptr = unsafe { PtrMut::new(slot_ptr) };

        Self {
            family_id: TypeId::of::<F>(),
            slot_ptr,
        }
    }

    pub fn take<F>(self) -> Option<F::Member<'a>>
    where
        F: Family,
    {
        // Make sure the protocol matches, which should give us a matching reference value
        if self.family_id != TypeId::of::<F>() {
            return None;
        }

        // Very unsafe, very bad, downcast the message
        let slot = unsafe { self.slot_ptr.deref_mut::<Option<F::Member<'a>>>() };

        // Take the value out
        slot.take()
    }
}
