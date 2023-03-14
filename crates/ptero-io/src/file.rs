use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use stewart::{Actor, Addr, After, Id, Options, System};
use tracing::{event, instrument, Level};

use crate::ReadWriteCmd;

/// Start a file actor implementation of the `ReadWriteCmd` message.
#[instrument("file-read-write", skip_all)]
pub fn start_file_read_write(
    system: &mut System,
    parent: Id,
    path: String,
    truncate: bool,
) -> Result<Addr<ReadWriteCmd>, Error> {
    let package_file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .open(path)
        .context("failed to open target package for writing")?;

    let info = system.create(parent)?;
    let actor = FileReadWriteActor { package_file };
    system.start(info, actor, Options::default())?;

    Ok(info.addr())
}

struct FileReadWriteActor {
    package_file: File,
}

impl Actor for FileReadWriteActor {
    type Message = ReadWriteCmd;

    fn handle<'a>(&mut self, system: &mut System, message: ReadWriteCmd) -> Result<After, Error> {
        event!(Level::INFO, "handling {}", message.kind());

        match message {
            ReadWriteCmd::Read {
                start,
                length,
                on_result,
            } => {
                let mut buffer = vec![0u8; length as usize];

                self.package_file.seek(SeekFrom::Start(start))?;
                self.package_file.read_exact(&mut buffer)?;

                system.send(on_result, buffer);
            }
            ReadWriteCmd::Write { start, data } => {
                self.package_file.seek(SeekFrom::Start(start))?;
                self.package_file.write_all(&data)?;
            }
        }
        Ok(After::Nothing)
    }
}
