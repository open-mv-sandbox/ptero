use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use ptero_daicon::io::ReadWriteCmd;
use stewart::{
    utils::{ActorT, AddrT, SystemExt},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, Level};

pub fn start_file_read_write(
    system: &mut System,
    path: String,
) -> Result<AddrT<ReadWriteCmd>, Error> {
    let addr = system.start_with("ppcli-rwfile", path, FileReadWriteActor::start)?;
    Ok(addr)
}

struct FileReadWriteActor {
    queue: Vec<ReadWriteCmd>,
    package_file: File,
    scratch_buffer: Vec<u8>,
}

impl FileReadWriteActor {
    fn start(
        _system: &mut System,
        _addr: AddrT<ReadWriteCmd>,
        path: String,
    ) -> Result<Self, Error> {
        let package_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .context("failed to open target package for writing")?;

        Ok(Self {
            queue: Vec::new(),
            package_file,
            scratch_buffer: Vec::new(),
        })
    }
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
