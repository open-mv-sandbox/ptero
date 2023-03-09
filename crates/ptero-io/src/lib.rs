mod file;

use anyhow::Error;
use family::{Family, Member};
use stewart::handler::Sender;

pub use self::file::start_file_read_write;

/// Read or write command.
pub enum ReadWriteCmd {
    Read {
        start: u64,
        length: u64,
        on_result: Sender<ReadResultF>,
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

pub type ReadResult<'a> = Result<&'a [u8], Error>;

pub enum ReadResultF {}

impl Family for ReadResultF {
    type Member<'a> = ReadResult<'a>;
}

impl<'a> Member<ReadResultF> for ReadResult<'a> {}
