mod file;

use stewart::Addr;

pub use self::file::start_file;

// TODO: Find a better option than sending around disposable mallocs, probably bumpalo.
// This will be changed once we've got something to profile most likely.

/// IO read command.
pub struct Read {
    pub start: u64,
    pub length: u64,
    pub on_result: Addr<ReadResult>,
}

/// IO write command.
pub struct Write {
    pub start: u64,
    pub data: Vec<u8>,
}

pub type ReadResult = Vec<u8>;
