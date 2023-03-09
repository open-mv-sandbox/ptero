//! Message dispatching and handling.

mod handler;
mod sender;

pub use self::{
    handler::{After, Handler, HandlerT},
    sender::{Sender, SenderT},
};
