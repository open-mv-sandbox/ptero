use anyhow::Error;
use family::{Family, Member};
use stewart::Addr;

/// Read or write command.
pub enum ReadWriteCmd {
    Read {
        start: u64,
        length: u64,
        reply: Addr<ReadResultF>,
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
