use anyhow::Error;
use stewart::{ActorAddr, Protocol};

#[derive(Protocol)]
pub enum ReadWrite {
    Read {
        start: u64,
        length: u64,
        reply: ActorAddr<ReadResult>,
    },
    Write {
        start: u64,
        data: Vec<u8>,
    },
}

// TODO: Figure out a better way than passing small vectors
// With the new stewart version we actually can do that now
#[derive(Protocol)]
pub struct ReadResult(pub Result<Vec<u8>, Error>);
