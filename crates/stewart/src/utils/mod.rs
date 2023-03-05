//! Small convenience utilities.

mod family;
mod system;
mod void;

pub use self::{
    family::{ActorAddrT, ActorT},
    system::SystemExt,
    void::VoidActor,
};
pub use ::void::Void;
