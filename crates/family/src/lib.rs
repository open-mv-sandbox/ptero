//! Small family pattern implementation.
//!
//! The family pattern implements "associated type constructors".
//!
//! See this post for more information:
//! <http://smallcultfollowing.com/babysteps/blog/2016/11/03/associated-type-constructors-part-2-family-traits/>

pub mod any;
pub mod utils;

/// Family pattern family interface.
pub trait Family: 'static {
    type Member<'a>;
}

/// Family pattern member interface.
pub trait Member {
    type Family: Family;
}
