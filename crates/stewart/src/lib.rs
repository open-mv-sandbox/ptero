mod actor;
mod addr;
mod dynamic;
mod system;

pub use self::{
    actor::{Actor, AfterReduce, Protocol},
    addr::{RawSystemAddr, SystemAddr},
    dynamic::{AnyActor, AnyMessage},
    system::System,
};
pub use stewart_derive::{Factory, Protocol};

pub trait Factory {
    fn start(self: Box<Self>, addr: RawSystemAddr) -> Box<dyn AnyActor>;
}
