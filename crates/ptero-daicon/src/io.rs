use anyhow::Error;
use family::{Family, Member};
use stewart::ActorAddr;

pub enum ReadWrite {
    Read {
        start: u64,
        length: u64,
        reply: ActorAddr<ReadResultF>,
    },
    Write {
        start: u64,
        data: Vec<u8>,
    },
}

impl ReadWrite {
    pub fn kind(&self) -> &'static str {
        match self {
            ReadWrite::Read { .. } => "read",
            ReadWrite::Write { .. } => "write",
        }
    }
}

pub type ReadResult<'a> = Result<&'a [u8], Error>;

pub enum ReadResultF {}

impl Family for ReadResultF {
    type Member<'a> = ReadResult<'a>;
}

impl<'a> Member<ReadResultF> for ReadResult<'a> {}
