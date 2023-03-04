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

pub use self::{
    actor::{Actor, AfterProcess, AfterReduce, Start},
    addr::{ActorAddr, ActorAddrF, AnyActorAddr},
    system::System,
};

/// Family pattern interface.
///
/// The family pattern implements "associated type constructors".
///
/// See this post for more information:
/// http://smallcultfollowing.com/babysteps/blog/2016/11/03/associated-type-constructors-part-2-family-traits/
pub trait Family {
    type Member<'a>;
}
