//! Small convenience utilities.

mod family;
mod map;
mod void;

pub use self::{
    family::{ActorT, AddrT},
    map::start_map,
    void::VoidActor,
};
pub use ::void::Void;
