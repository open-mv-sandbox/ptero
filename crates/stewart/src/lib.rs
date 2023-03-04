//! Stewart is a modular, flexible, and high-performance actor system.
//!
//! This is an API reference for the stewart rust library. For a detailed user guide, read the
//! stewart book.

mod actor;
mod addr;
mod dynamic;
mod factory;
mod system;
pub mod utils;

use std::marker::PhantomData;

pub use self::{
    actor::{Actor, ActorF, AfterProcess, AfterReduce, Start},
    addr::{ActorAddr, ActorAddrF},
    system::System,
};

/// Family pattern interface.
///
/// The family pattern implements "associated type constructors".
///
/// See this post for more information:
/// <http://smallcultfollowing.com/babysteps/blog/2016/11/03/associated-type-constructors-part-2-family-traits/>
pub trait Family: 'static {
    type Member<'a>;
}

/// Generic family for types where every member has the same static lifetime.
pub struct StaticFamily<T> {
    _t: PhantomData<T>,
}

impl<T: 'static> Family for StaticFamily<T> {
    type Member<'a> = T;
}
