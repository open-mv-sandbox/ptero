use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read as IoRead, Seek, SeekFrom, Write as IoWrite},
};

use anyhow::{Context as ContextExt, Error};
use stewart::{Actor, Addr, After, Context, Options};
use tracing::{event, instrument, Level};

use crate::{FileMessage, Operation, ReadResult, WriteLocation, WriteResult};

/// Start a file reader/writer from a system file.
#[instrument("file", skip_all)]
pub fn start_system_file(
    ctx: &mut Context,
    path: &str,
    truncate: bool,
) -> Result<Addr<FileMessage>, Error> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(truncate)
        .open(path)
        .context("failed to open system file for writing")?;

    let mut ctx = ctx.create()?;
    let actor = FileActor { file };
    ctx.start(Options::default(), actor)?;

    Ok(ctx.addr()?)
}

struct FileActor {
    file: File,
}

impl Actor for FileActor {
    type Message = FileMessage;

    fn handle(&mut self, ctx: &mut Context, message: FileMessage) -> Result<After, Error> {
        event!(Level::INFO, "handling message");

        match message.operation {
            Operation::Read {
                offset,
                size,
                on_result,
            } => {
                // TODO: Currently remaining bytes after EOF are kept zero, but maybe we want to
                // feedback a lack of remaining bytes.

                let mut data = vec![0u8; size as usize];

                self.file.seek(SeekFrom::Start(offset))?;
                read_exact_eof(&mut self.file, &mut data)?;

                // Reply result
                let result = ReadResult {
                    id: message.id,
                    data,
                };
                ctx.send(on_result, result);
            }
            Operation::Write {
                location,
                data,
                on_result,
            } => {
                // Seek to given location
                let from = match location {
                    WriteLocation::Offset(offset) => SeekFrom::Start(offset),
                    WriteLocation::Append => SeekFrom::End(0),
                };
                self.file.seek(from)?;
                let offset = self.file.stream_position()?;

                // Perform the write
                self.file.write_all(&data)?;

                // Reply result
                let result = WriteResult {
                    id: message.id,
                    offset,
                };
                ctx.send(on_result, result);
            }
        }
        Ok(After::Continue)
    }
}

/// Copy of read_exact except allowing for EOF.
fn read_exact_eof(file: &mut File, mut buf: &mut [u8]) -> Result<(), Error> {
    while !buf.is_empty() {
        match file.read(buf) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(error) => match error.kind() {
                ErrorKind::Interrupted => {}
                ErrorKind::UnexpectedEof => break,
                _ => return Err(error.into()),
            },
        }
    }

    Ok(())
}
