//! Message dispatching and handling.

mod handler;
mod sender;

pub use self::{
    handler::{Handler, HandlerT},
    sender::{Map, Sender, SenderT},
};
