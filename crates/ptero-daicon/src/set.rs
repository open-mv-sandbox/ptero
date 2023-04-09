use anyhow::Error;
use bytemuck::{bytes_of, Zeroable};
use daicon::Entry;
use ptero_file::{FileMessage, Operation, WriteLocation, WriteResult};
use stewart::{Actor, Addr, After, Context, Options};
use stewart_utils::MapExt;
use tracing::{event, instrument, Level};
use uuid::Uuid;

#[instrument("set-task", skip_all)]
pub fn start_set_task(
    ctx: &mut Context,
    file: Addr<FileMessage>,
    id: Uuid,
    data: Vec<u8>,
    on_result: Addr<()>,
) -> Result<Addr<u64>, Error> {
    let mut ctx = ctx.create()?;
    let addr = ctx.addr()?;

    // Start the append immediately
    let size = data.len() as u64;
    let message = FileMessage {
        id: Uuid::new_v4(),
        operation: Operation::Write {
            location: WriteLocation::Append,
            data,
            on_result: ctx.map(addr, Message::AppendResult)?,
        },
    };
    ctx.send(file, message);

    // Create the actor for tracking state of writing
    let mut entry = Entry::zeroed();
    entry.set_id(id);
    entry.set_size(size);
    let task = SetTask {
        file,
        on_result,

        entry_offset: None,
        data_offset: None,
        entry,
    };
    ctx.start(Options::default(), task)?;

    Ok(ctx.map_once(addr, Message::Slot)?)
}

struct SetTask {
    file: Addr<FileMessage>,
    on_result: Addr<()>,

    entry_offset: Option<u64>,
    data_offset: Option<u64>,
    entry: Entry,
}

impl Actor for SetTask {
    type Message = Message;

    fn handle(&mut self, ctx: &mut Context, message: Message) -> Result<After, Error> {
        match message {
            Message::Slot(offset) => {
                self.entry_offset = Some(offset);
            }
            Message::AppendResult(message) => {
                self.data_offset = Some(message.offset);
            }
            Message::EntryResult(_message) => {
                // TODO: Report valid back to service, and only after that's flushed report result

                event!(Level::DEBUG, "success, sending result");

                ctx.send(self.on_result, ());
                return Ok(After::Stop);
            }
        };

        // If we got both parts we need to write entry, do so
        if let (Some(entry_offset), Some(data_offset)) = (self.entry_offset, self.data_offset) {
            event!(Level::DEBUG, "writing table entry");

            // Complete the entry
            self.entry.set_offset(data_offset);

            // Write the entry to the slot we got
            let message = FileMessage {
                id: Uuid::new_v4(),
                operation: Operation::Write {
                    location: WriteLocation::Offset(entry_offset),
                    data: bytes_of(&self.entry).to_owned(),
                    on_result: ctx.map_once(ctx.addr()?, Message::EntryResult)?,
                },
            };
            ctx.send(self.file, message);
        }

        Ok(After::Continue)
    }
}

enum Message {
    Slot(u64),
    AppendResult(WriteResult),
    EntryResult(WriteResult),
}
