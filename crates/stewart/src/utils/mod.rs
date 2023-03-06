//! Small convenience utilities.

mod family;
mod map;
mod system;
mod void;

pub use self::{
    family::{AddrT, ActorT},
    map::start_map,
    system::SystemExt,
    void::VoidActor,
};
pub use ::void::Void;
