mod file;

use stewart::Addr;

pub use self::file::start_file_read_write;

/// Read or write command.
pub enum ReadWriteCmd {
    Read {
        start: u64,
        length: u64,
        on_result: Addr<ReadResult>,
    },
    Write {
        start: u64,
        data: Vec<u8>,
    },
}

impl ReadWriteCmd {
    pub fn kind(&self) -> &'static str {
        match self {
            ReadWriteCmd::Read { .. } => "read",
            ReadWriteCmd::Write { .. } => "write",
        }
    }
}

// TODO: Get a better option than sending around disposable vecs.
pub type ReadResult = Vec<u8>;
