//! Small family pattern implementation.
//!
//! The family pattern implements "associated type constructors".
//!
//! See this post for more information:
//! <http://smallcultfollowing.com/babysteps/blog/2016/11/03/associated-type-constructors-part-2-family-traits/>

use std::marker::PhantomData;

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
