use anyhow::Error;
use stewart::{Actor, Addr, After, Context, Id, Messages, Options, System};
use tracing::{event, instrument, Level};

use crate::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};

/// Start a file service from a buffer.
#[instrument("buffer-file", skip_all)]
pub fn open_buffer(ctx: &mut Context, buffer: Vec<u8>) -> Result<Addr<FileMessage>, Error> {
    let (id, mut ctx) = ctx.create()?;
    let actor = BufferFileService { buffer };
    ctx.start(id, Options::default(), actor)?;

    Ok(Addr::new(id))
}

struct BufferFileService {
    buffer: Vec<u8>,
}

impl BufferFileService {
    fn handle(&mut self, system: &mut System, message: FileMessage) {
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
                system.send(on_result, result);
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
                system.send(on_result, result);
            }
        }
    }
}

impl Actor for BufferFileService {
    type Message = FileMessage;

    fn process(
        &mut self,
        system: &mut System,
        _id: Id,
        messages: &mut Messages<FileMessage>,
    ) -> Result<After, Error> {
        event!(Level::INFO, "handling message");

        while let Some(message) = messages.next() {
            self.handle(system, message);
        }

        Ok(After::Continue)
    }
}
