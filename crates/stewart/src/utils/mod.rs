//! Small convenience utilities.

// TODO: Move these to own crate

mod family;
mod map;

pub use self::{
    family::{ActorT, AddrT},
    map::start_map,
};
