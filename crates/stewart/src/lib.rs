mod actor;
mod addr;
mod dynamic;
mod system;

pub use self::{
    actor::{Actor, AfterReduce},
    addr::{AnySystemAddr, SystemAddr},
    dynamic::AnyActor,
    system::System,
};
pub use stewart_derive::Factory;

pub trait Factory {
    fn start(self: Box<Self>, addr: AnySystemAddr) -> Box<dyn AnyActor>;
}
