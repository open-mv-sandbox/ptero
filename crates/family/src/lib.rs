//! Small family pattern implementation.
//!
//! The family pattern implements "associated type constructors".
//!
//! See this post for more information:
//! <http://smallcultfollowing.com/babysteps/blog/2016/11/03/associated-type-constructors-part-2-family-traits/>

use std::{any::TypeId, marker::PhantomData, ptr::NonNull};

/// Family pattern interface.
pub trait Family: 'static {
    type Member<'a>;
}

/// Generic family for types where every member has the same static lifetime.
///
/// Workaround for a lack of HKT support in Rust.
pub struct StaticFamily<T> {
    _t: PhantomData<T>,
}

impl<T: 'static> Family for StaticFamily<T> {
    type Member<'a> = T;
}

/// Reference to an Option that can contain any `Family::Member`, checked at runtime.
pub struct AnyOptionMut<'a> {
    family_id: TypeId,
    slot_ptr: NonNull<u8>,
    _a: PhantomData<&'a mut u32>,
}

impl<'a> AnyOptionMut<'a> {
    pub fn new<'b, F>(slot: &'a mut Option<F::Member<'b>>) -> Self
    where
        'b: 'a,
        F: Family,
    {
        let slot_ptr = NonNull::new(slot).unwrap().cast();

        Self {
            family_id: TypeId::of::<F>(),
            slot_ptr,
            _a: PhantomData,
        }
    }

    pub fn downcast<F>(self) -> Option<&'a mut Option<F::Member<'a>>>
    where
        F: Family,
    {
        // Make sure the family matches, which should give us a matching reference value
        if self.family_id != TypeId::of::<F>() {
            return None;
        }

        // Very unsafe, very bad, downcast the inner option
        let mut slot_ptr_typed: NonNull<Option<F::Member<'a>>> = self.slot_ptr.cast();
        Some(unsafe { slot_ptr_typed.as_mut() })
    }
}
