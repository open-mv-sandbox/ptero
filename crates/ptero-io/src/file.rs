use std::{
    fs::{File, OpenOptions},
    io::{Read as IoRead, Seek, SeekFrom, Write as IoWrite},
};

use anyhow::{Context as ContextExt, Error};
use stewart::{Actor, Addr, After, Id, Options, System};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};

use crate::{Read, Write};

/// Start a file implementation of `Read` and `Write` messages.
#[instrument("file", skip_all)]
pub fn start_file(
    system: &mut System,
    parent: Id,
    path: String,
    truncate: bool,
) -> Result<(Addr<Read>, Addr<Write>), Error> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .open(path)
        .context("failed to open target package for writing")?;

    let info = system.create(parent)?;
    let actor = FileActor { file };
    system.start(info, Options::default(), actor)?;

    // Individual read/write
    let read = start_map(system, info.id(), info.addr(), FileCommand::Read)?;
    let write = start_map(system, info.id(), info.addr(), FileCommand::Write)?;

    Ok((read, write))
}

struct FileActor {
    file: File,
}

impl Actor for FileActor {
    type Message = FileCommand;

    fn handle<'a>(&mut self, system: &mut System, message: FileCommand) -> Result<After, Error> {
        event!(Level::INFO, kind = message.kind(), "handling command");

        match message {
            FileCommand::Read(command) => {
                let mut buffer = vec![0u8; command.length as usize];

                self.file.seek(SeekFrom::Start(command.start))?;
                self.file.read_exact(&mut buffer)?;

                system.send(command.on_result, buffer);
            }
            FileCommand::Write(command) => {
                self.file.seek(SeekFrom::Start(command.start))?;
                self.file.write_all(&command.data)?;
            }
        }
        Ok(After::Nothing)
    }
}

enum FileCommand {
    Read(Read),
    Write(Write),
}

impl FileCommand {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Read(_) => "read",
            Self::Write(_) => "write",
        }
    }
}
