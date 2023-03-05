use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use ptero_daicon::io::ReadWrite;
use stewart::{
    utils::{ActorAddrT, ActorT, SystemExt},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, Level};

pub fn start_read_write_file(system: &mut System, data: FileReadWrite) {
    system.start_with("ppcli-rwfile", data, FileReadWriteActor::start);
}

pub struct FileReadWrite {
    pub path: String,
    pub reply: ActorAddrT<ActorAddrT<ReadWrite>>,
}

struct FileReadWriteActor {
    queue: Vec<ReadWrite>,
    package_file: File,
    scratch_buffer: Vec<u8>,
}

impl FileReadWriteActor {
    fn start(
        system: &mut System,
        addr: ActorAddrT<ReadWrite>,
        data: FileReadWrite,
    ) -> Result<Self, Error> {
        let package_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(data.path)
            .context("failed to open target package for writing")?;

        system.handle(data.reply, addr);

        Ok(Self {
            queue: Vec::new(),
            package_file,
            scratch_buffer: Vec::new(),
        })
    }
}

impl ActorT for FileReadWriteActor {
    type Message = ReadWrite;

    fn reduce<'a>(&mut self, message: ReadWrite) -> Result<AfterReduce, Error> {
        self.queue.push(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        event!(
            Level::DEBUG,
            count = self.queue.len(),
            "processing operations"
        );

        for message in self.queue.drain(..) {
            event!(Level::DEBUG, "performing {}", message.kind());

            match message {
                ReadWrite::Read {
                    start,
                    length,
                    reply,
                } => {
                    self.scratch_buffer.resize(length as usize, 0);

                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.read_exact(&mut self.scratch_buffer)?;
                    let msg = Ok(self.scratch_buffer.as_slice());
                    system.handle(reply, msg);
                }
                ReadWrite::Write { start, data } => {
                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.write_all(&data)?;
                }
            }
        }

        Ok(AfterProcess::Nothing)
    }
}
