//! Pterodactil file abstraction.
//!
//! A "file" is an addressable blob of binary data.

mod system;

use stewart::Addr;

pub use self::system::start_system_file;

/// Operation on a file.
pub enum FileMessage {
    Read {
        offset: u64,
        size: u64,
        on_result: Addr<Vec<u8>>,
    },
    Write {
        offset: u64,
        data: Vec<u8>,
    },
}
