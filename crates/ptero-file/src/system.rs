use std::{
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom, Write},
};

use anyhow::{Context as _, Error};
use stewart::{Addr, State, System, SystemId, SystemOptions, World};
use stewart_utils::Context;
use tracing::{event, instrument, Level};

use crate::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};

/// System file API entry point.
#[derive(Clone)]
pub struct SystemFileApi {
    system: SystemId,
}

impl SystemFileApi {
    pub fn new(world: &mut World) -> Self {
        Self {
            system: world.register(SystemOptions::default(), SystemFileSystem),
        }
    }

    #[instrument("system-file", skip_all)]
    pub fn open(
        &self,
        ctx: &mut Context,
        path: &str,
        truncate: bool,
    ) -> Result<Addr<FileMessage>, Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(truncate)
            .create(true)
            .open(path)
            .context("failed to open system file for writing")?;

        let (id, mut ctx) = ctx.create(self.system)?;
        let instance = SystemFile { file };
        ctx.start(id, instance)?;

        Ok(Addr::new(id))
    }
}

struct SystemFileSystem;

impl System for SystemFileSystem {
    type Instance = SystemFile;
    type Message = FileMessage;

    #[instrument("system-file", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        event!(Level::INFO, "handling messages");

        while let Some((actor, message)) = state.next() {
            let instance = state.get_mut(actor).context("failed to get instance")?;

            match message.action {
                FileAction::Read {
                    offset,
                    size,
                    on_result,
                } => {
                    // TODO: Currently remaining bytes after EOF are kept zero, but maybe we want to
                    // feedback a lack of remaining bytes.

                    let mut data = vec![0u8; size as usize];

                    instance.file.seek(SeekFrom::Start(offset))?;
                    read_exact_eof(&mut instance.file, &mut data)?;

                    // Reply result
                    let result = ReadResult {
                        id: message.id,
                        offset,
                        data,
                    };
                    world.send(on_result, result);
                }
                FileAction::Write {
                    location,
                    data,
                    on_result,
                } => {
                    // Seek to given location
                    let from = match location {
                        WriteLocation::Offset(offset) => SeekFrom::Start(offset),
                        WriteLocation::Append => SeekFrom::End(0),
                    };
                    instance.file.seek(from)?;
                    let offset = instance.file.stream_position()?;

                    // Perform the write
                    instance.file.write_all(&data)?;

                    // Reply result
                    let result = WriteResult {
                        id: message.id,
                        offset,
                    };
                    world.send(on_result, result);
                }
            }
        }

        Ok(())
    }
}

struct SystemFile {
    file: File,
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
