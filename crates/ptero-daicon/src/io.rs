use anyhow::Error;
use stewart::utils::ActorAddrS;

pub enum ReadWrite {
    Read {
        start: u64,
        length: u64,
        reply: ActorAddrS<ReadResult>,
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

// TODO: Figure out a better way than passing small vectors
// With the new stewart version we actually can do that now
pub struct ReadResult(pub Result<Vec<u8>, Error>);
