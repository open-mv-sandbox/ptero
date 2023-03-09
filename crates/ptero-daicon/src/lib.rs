//! Pterodactil implementation of the "Daicon" format.

mod manager;
mod read;

pub use self::manager::{
    start_file_manager, FileManagerCommand, GetComponentCommand, GetComponentResult,
};
