use std::{
    collections::VecDeque,
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
    queue: VecDeque<ReadWrite>,
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
            queue: VecDeque::new(),
            package_file,
        })
    }
}

impl Actor for FileReadWriteActor {
    type Protocol = ReadWrite;

    fn reduce<'a>(&mut self, message: ReadWrite) -> Result<AfterReduce, Error> {
        self.queue.push_back(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        while let Some(message) = self.queue.pop_front() {
            match message {
                ReadWrite::Read {
                    start,
                    length,
                    reply,
                } => {
                    event!(Level::DEBUG, "performing read");
                    let mut buffer = vec![0u8; length as usize];
                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.read_exact(&mut buffer)?;
                    system.handle(reply, ReadResult(Ok(buffer)));
                }
                ReadWrite::Write { start, data } => {
                    event!(Level::DEBUG, "performing write");
                    self.package_file.seek(SeekFrom::Start(start))?;
                    self.package_file.write_all(&data)?;
                }
            }
        }

        Ok(AfterProcess::Nothing)
    }
}
