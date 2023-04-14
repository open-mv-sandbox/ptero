use anyhow::{Context as _, Error};
use stewart::{Addr, State, System, SystemId, SystemOptions, World};
use stewart_utils::Context;
use tracing::{event, instrument, Level};

use crate::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};

/// Buffer file API entry point.
#[derive(Clone)]
pub struct BufferFileApi {
    system: SystemId,
}

impl BufferFileApi {
    pub fn new(world: &mut World) -> Self {
        Self {
            system: world.register(SystemOptions::default(), BufferFileSystem),
        }
    }

    #[instrument("buffer-file", skip_all)]
    pub fn open_buffer(
        &self,
        ctx: &mut Context,
        buffer: Vec<u8>,
    ) -> Result<Addr<FileMessage>, Error> {
        let (id, mut ctx) = ctx.create(self.system)?;
        let instance = BufferFile { buffer };
        ctx.start(id, instance)?;

        Ok(Addr::new(id))
    }
}

struct BufferFileSystem;

impl System for BufferFileSystem {
    type Instance = BufferFile;
    type Message = FileMessage;

    #[instrument("buffer-file", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        event!(Level::INFO, "handling message");

        while let Some((id, message)) = state.next() {
            let instance = state.get_mut(id).context("failed to get instance")?;
            instance.handle(world, message);
        }

        Ok(())
    }
}

struct BufferFile {
    buffer: Vec<u8>,
}

impl BufferFile {
    fn handle(&mut self, world: &mut World, message: FileMessage) {
        match message.action {
            FileAction::Read {
                offset,
                size,
                on_result,
            } => {
                // TODO: Currently remaining bytes after EOF are kept zero, but maybe we want to
                // feedback a lack of remaining bytes.

                let offset = offset as usize;
                let mut data = vec![0u8; size as usize];

                let available = self.buffer.len() - offset;
                let slice_len = usize::min(data.len(), available);

                let src = &self.buffer[offset..offset + slice_len];
                let dst = &mut data[0..slice_len];

                dst.copy_from_slice(src);

                // Reply result
                let result = ReadResult {
                    id: message.id,
                    offset: offset as u64,
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
                let offset = match location {
                    WriteLocation::Offset(offset) => offset as usize,
                    WriteLocation::Append => data.len(),
                };

                // Overwrite what's already there
                let available = self.buffer.len() - offset as usize;
                let src = &data[0..available];
                let dst = &mut self.buffer[offset..offset + available];
                dst.copy_from_slice(src);

                // Append the rest
                self.buffer.extend_from_slice(&data[available..]);

                // Reply result
                let result = WriteResult {
                    id: message.id,
                    offset: offset as u64,
                };
                world.send(on_result, result);
            }
        }
    }
}
