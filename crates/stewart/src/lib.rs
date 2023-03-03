mod actor;
mod addr;
pub mod dynamic;
mod system;
pub mod utils;

pub use self::{
    actor::{Actor, AfterReduce, Protocol},
    addr::{ActorAddr, RawAddr},
    dynamic::{AnyActor, AnyMessage},
    system::System,
};
pub use stewart_derive::{Factory, Protocol};

pub trait Factory {
    fn start(self: Box<Self>, addr: RawAddr) -> Box<dyn AnyActor>;
}
