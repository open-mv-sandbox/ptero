use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use stewart::{
    utils::{ActorT, AddrT},
    ActorId, AfterProcess, AfterReduce, System,
};
use tracing::{event, instrument, Level};

use crate::ReadWriteCmd;

/// Start a file actor implementation of the `ReadWriteCmd` message.
#[instrument("file-read-write", skip_all)]
pub fn start_file_read_write(
    system: &mut System,
    parent: ActorId,
    path: String,
    truncate: bool,
) -> Result<AddrT<ReadWriteCmd>, Error> {
    let (_, addr) = system.create_addr(parent)?;

    let package_file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .open(path)
        .context("failed to open target package for writing")?;

    let actor = FileReadWriteActor {
        queue: Vec::new(),
        package_file,
        scratch_buffer: Vec::new(),
    };
    system.start(addr, actor)?;

    Ok(addr)
}

struct FileReadWriteActor {
    queue: Vec<ReadWriteCmd>,
    package_file: File,
    scratch_buffer: Vec<u8>,
}

impl ActorT for FileReadWriteActor {
    type Message = ReadWriteCmd;

    fn reduce<'a>(&mut self, message: ReadWriteCmd) -> Result<AfterReduce, Error> {
        self.queue.push(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        event!(
            Level::INFO,
            count = self.queue.len(),
            "processing operations"
        );

        for message in self.queue.drain(..) {
            event!(Level::INFO, "performing {}", message.kind());

            match message {
                ReadWriteCmd::Read {
                    start,
                    length,
                    reply,
                } => {
                    self.scratch_buffer.resize(length as usize, 0);

                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.read_exact(&mut self.scratch_buffer)?;
                    let msg = Ok(self.scratch_buffer.as_slice());
                    system.handle(reply, msg)?;
                }
                ReadWriteCmd::Write { start, data } => {
                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.write_all(&data)?;
                }
            }
        }

        Ok(AfterProcess::Nothing)
    }
}
