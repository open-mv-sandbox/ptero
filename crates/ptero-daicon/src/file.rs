use std::io::Write;

use anyhow::Error;
use bytemuck::{bytes_of, Zeroable};
use daicon::Entry;
use ptero_file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};
use stewart::{Actor, Addr, After, Context, Options};
use stewart_utils::{MapExt, WhenExt};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{cache::CachedTable, set::start_set_task, SourceAction, SourceMessage};

/// Open a file as a daicon source.
///
/// A "source" returns a file from UUIDs. A "file source" uses a file as a source.
#[instrument("file-source-service", skip_all)]
pub fn open_file(
    ctx: &mut Context,
    file: Addr<FileMessage>,
    create: bool,
) -> Result<Addr<SourceMessage>, Error> {
    let mut ctx = ctx.create()?;
    let addr = ctx.addr()?;

    let source = ctx.map(addr, Message::SourceMessage)?;
    let mut table = None;

    // TODO: create bool's a bit messy but it's a start
    if create {
        let create_table = CachedTable::new(0, 256);
        let mut data = Vec::new();

        // Write the header
        let (header, _) = create_table.create_header();
        data.write_all(bytes_of(&header))?;

        // Write empty entries
        for _ in 0..256 {
            let entry = Entry::zeroed();
            data.write_all(bytes_of(&entry))?;
        }

        // Send to file for writing
        let action = FileAction::Write {
            location: WriteLocation::Offset(0),
            data,
            on_result: ctx.when(|_, _| Ok(After::Stop))?,
        };
        let message = FileMessage {
            id: Uuid::new_v4(),
            action,
        };
        ctx.send(file, message);

        // Store the table
        table = Some(create_table);
    } else {
        // Immediately start table read
        let message = FileMessage {
            id: Uuid::new_v4(),
            action: FileAction::Read {
                offset: 0,
                size: 64 * 1024,
                on_result: ctx.map_once(addr, Message::ReadTableResult)?,
            },
        };
        ctx.send(file, message);
    }

    // Start the root manager actor
    let actor = FileSourceService {
        write_header_result: ctx.map(addr, Message::WriteHeaderResult)?,
        file,
        table,

        get_tasks: Vec::new(),
        pending_slots: Vec::new(),
    };
    ctx.start(Options::default(), actor)?;

    Ok(source)
}

struct FileSourceService {
    write_header_result: Addr<WriteResult>,
    file: Addr<FileMessage>,
    table: Option<CachedTable>,

    pending_slots: Vec<Addr<u64>>,

    // TODO: Stateful temporary tasks should also be actors, most of set already is
    get_tasks: Vec<(Uuid, Addr<ReadResult>)>,
}

impl FileSourceService {
    fn on_source_message(
        &mut self,
        ctx: &mut Context,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message.action {
            SourceAction::Get { id, on_result } => {
                event!(Level::INFO, ?id, "received get");
                self.get_tasks.push((id, on_result));
            }
            SourceAction::Set {
                id,
                data,
                on_result,
            } => {
                event!(Level::INFO, ?id, bytes = data.len(), "received set");

                let addr = start_set_task(ctx, self.file, id, data, on_result)?;
                self.pending_slots.push(addr);
            }
        }

        Ok(())
    }

    fn on_read_table(&mut self, result: ReadResult) -> Result<(), Error> {
        let table = CachedTable::read(result.offset, result.data)?;
        self.table = Some(table);

        // TODO: Follow additional headers

        Ok(())
    }

    fn on_write(&mut self, _result: WriteResult) -> Result<(), Error> {
        // TODO: Report back entry valid once it falls in the header's valid range

        Ok(())
    }

    fn check_pending(&mut self, ctx: &mut Context) {
        let table = if let Some(table) = self.table.as_mut() {
            table
        } else {
            return;
        };

        // Resolve pending gets
        self.get_tasks
            .retain(|(id, on_result)| !try_read_data(ctx, self.file, table, *id, *on_result));

        // Resolve pending sets
        self.pending_slots.retain(|on_result| {
            !try_allocate_slot(ctx, self.write_header_result, self.file, table, *on_result)
        });

        // TODO: Reply failure if we ran out of tables to read, and couldn't find it
        // TODO: Allocate new tables if we ran out of free spaces
    }
}

impl Actor for FileSourceService {
    type Message = Message;

    fn handle(&mut self, ctx: &mut Context, message: Message) -> Result<After, Error> {
        match message {
            Message::SourceMessage(message) => {
                self.on_source_message(ctx, message)?;
            }
            Message::ReadTableResult(result) => {
                self.on_read_table(result)?;
            }
            Message::WriteHeaderResult(result) => {
                self.on_write(result)?;
            }
        };

        // Check if we can resolve any get requests
        self.check_pending(ctx);

        Ok(After::Continue)
    }
}

enum Message {
    SourceMessage(SourceMessage),
    ReadTableResult(ReadResult),
    WriteHeaderResult(WriteResult),
}

fn try_read_data(
    ctx: &mut Context,
    file: Addr<FileMessage>,
    table: &mut CachedTable,
    id: Uuid,
    on_result: Addr<ReadResult>,
) -> bool {
    let entry = if let Some(entry) = table.find(id) {
        entry
    } else {
        return false;
    };

    event!(
        Level::INFO,
        id = ?entry.id(),
        "found entry"
    );

    // We found a matching entry, start the read to fetch the inner data
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Read {
            offset: entry.offset(),
            size: entry.size(),
            on_result,
        },
    };
    ctx.send(file, message);

    true
}

fn try_allocate_slot(
    ctx: &mut Context,
    write_header_result: Addr<WriteResult>,
    file: Addr<FileMessage>,
    table: &mut CachedTable,
    on_result: Addr<u64>,
) -> bool {
    let index = if let Some(index) = table.try_allocate() {
        index
    } else {
        return false;
    };

    // Reply that we've found a slot
    let offset = table.entry_offset(index);
    ctx.send(on_result, offset);

    // Write the new header with the updated valid count
    // TODO: Wait until the task tells us to validate
    // TODO: Get the entry back from the task, currently the cache is wrong
    table.mark_valid(index, Entry::default());
    let (header, offset) = table.create_header();
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Offset(offset),
            data: bytes_of(&header).to_owned(),
            on_result: write_header_result,
        },
    };
    ctx.send(file, message);

    true
}
