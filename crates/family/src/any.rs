//! Dynamic type casting support for families.
//!
//! Rust std's `Any` type is limited to only work on `'static` types.
//! The limitations added by the family pattern allow it to do more safe casting.

use std::{any::TypeId, marker::PhantomData, ptr::NonNull};

use crate::Family;

// TODO: Replace `AnyOptionMut` below, with a family AnyMember caster.

/// Mutable reference to an `Option` that can contain any `Family::Member`, checked at runtime.
///
/// ```
/// # use family::*;
/// # enum StrFamily {}
/// # impl Family for StrFamily { type Member<'a> = &'a str; }
/// #
/// let value = String::from("some value");
/// let mut option = Some(value.as_str());
/// let option_mut = AnyOptionMut::new::<StrFamily>(&mut option);
///
/// // Dynamically downcast back to the option
/// let result = option_mut.downcast::<StrFamily>();
/// assert!(result.is_some());
/// let taken = result.unwrap().take();
/// assert!(taken.is_some());
///
/// // Afterwards, we can take the option back again
/// assert!(option.is_none());
/// ```
///
/// ```compile_fail
/// # use family::*;
/// # enum StrFamily {}
/// # impl Family for StrFamily { type Member<'a> = &'a str; }
/// #
/// let value = String::from("some value");
/// let mut option = Some(value.as_str());
/// let option_mut = AnyOptionMut::new::<StrFamily>(&mut option);
///
/// // Option is no longer valid
/// drop(option);
///
/// // So this fails to compile
/// option_mut.downcast::<StrFamily>();
/// ```
///
/// ```compile_fail
/// # use family::*;
/// # enum StrFamily {}
/// # impl Family for StrFamily { type Member<'a> = &'a str; }
/// #
/// let value = String::from("some value");
/// let mut option = Some(value.as_str());
/// let option_mut = AnyOptionMut::new::<StrFamily>(&mut option);
///
/// let result = option_mut.downcast::<StrFamily>();
///
/// // Option is no longer valid
/// drop(option);
///
/// // So this fails to compile
/// assert!(result.is_some());
/// ```
pub struct AnyOptionMut<'a> {
    family_id: TypeId,
    slot_ptr: NonNull<u8>,
    _a: PhantomData<&'a mut u32>,
}

impl<'a> AnyOptionMut<'a> {
    /// Create a new mutable reference to an option with a family member.
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
