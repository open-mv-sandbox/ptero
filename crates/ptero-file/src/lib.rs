//! Pterodactil file abstraction.
//!
//! A "file" is an addressable blob of binary data.

mod system;

use stewart::Addr;
use uuid::Uuid;

pub use self::system::start_system_file;

/// Message to a file actor.
pub struct FileMessage {
    /// Unique tracking identifier.
    pub id: Uuid,
    pub operation: Operation,
}

/// Operation to perform on a file.
pub enum Operation {
    Read {
        offset: u64,
        size: u64,
        on_result: Addr<ReadResult>,
    },
    Write {
        location: WriteLocation,
        data: Vec<u8>,
        on_result: Addr<WriteResult>,
    },
}

/// Location for `Operation::Write`.
pub enum WriteLocation {
    Offset(u64),
    Append,
}

/// Result of `Operation::Read`.
pub struct ReadResult {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Read data.
    pub data: Vec<u8>,
}

/// Result of `Operation::Write`.
pub struct WriteResult {
    /// Identifier of originating message.
    pub id: Uuid,
    /// Resolved stream offset written to.
    pub offset: u64,
}
