use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use ptero_daicon::io::{ReadResult, ReadWrite};
use stewart::{Actor, ActorAddr, AfterProcess, AfterReduce, Factory, System};
use tracing::{event, Level};

#[derive(Factory)]
#[factory(FileReadWriteActor::start)]
pub struct FileReadWrite {
    pub path: String,
    pub reply: ActorAddr<ActorAddr<ReadWrite>>,
}

struct FileReadWriteActor {
    queue: Vec<ReadWrite>,
    package_file: File,
}

impl FileReadWriteActor {
    pub fn start(
        system: &mut System,
        addr: ActorAddr<ReadWrite>,
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
        })
    }
}

impl Actor for FileReadWriteActor {
    type Protocol = ReadWrite;

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
                    let mut buffer = vec![0u8; length as usize];
                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.read_exact(&mut buffer)?;
                    system.handle(reply, ReadResult(Ok(buffer)));
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
