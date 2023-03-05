//! Small family pattern implementation.
//!
//! The family pattern implements "associated type constructors".
//!
//! See this post for more information:
//! <http://smallcultfollowing.com/babysteps/blog/2016/11/03/associated-type-constructors-part-2-family-traits/>

use std::any::Any;

pub mod any;
pub mod utils;

/// Family pattern family interface.
pub trait Family: Any + Sized {
    type Member<'a>: Member<Self>;
}

/// Family pattern member interface.
pub trait Member<F>
where
    F: Family,
{
}
