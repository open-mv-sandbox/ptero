use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use ptero_daicon::io::ReadWrite;
use stewart::{Actor, Address, Context, Factory, Next};
use tracing::{event, Level};

#[derive(Factory)]
#[factory(FileReadWriteActor::start)]
pub struct FileReadWrite {
    pub path: String,
    pub reply: Address<Address<ReadWrite>>,
}

struct FileReadWriteActor {
    package_file: File,
}

impl FileReadWriteActor {
    pub fn start(
        ctx: &dyn Context,
        address: Address<ReadWrite>,
        data: FileReadWrite,
    ) -> Result<Self, Error> {
        let package_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(data.path)
            .context("failed to open target package for writing")?;

        ctx.send(data.reply, address);

        Ok(Self { package_file })
    }
}

impl Actor for FileReadWriteActor {
    type Message = ReadWrite;

    fn handle(&mut self, ctx: &dyn Context, message: ReadWrite) -> Result<Next, Error> {
        match message {
            ReadWrite::Read {
                start,
                length,
                reply,
            } => {
                event!(Level::TRACE, "performing read");
                let mut buffer = vec![0u8; length as usize];
                self.package_file.seek(SeekFrom::Start(start))?;
                self.package_file.read_exact(&mut buffer)?;
                ctx.send(reply, Ok(buffer));
            }
            ReadWrite::Write { start, data } => {
                event!(Level::TRACE, "performing write");
                self.package_file.seek(SeekFrom::Start(start))?;
                self.package_file.write_all(&data)?;
            }
        }

        Ok(Next::Continue)
    }
}
