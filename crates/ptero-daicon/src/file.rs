use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon::{Entry, Header};
use ptero_file::{FileAction, FileMessage, ReadResult, WriteLocation, WriteResult};
use stewart::{Actor, Addr, After, Context, Options};
use stewart_utils::MapExt;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{set::start_set_task, SourceAction, SourceMessage};

/// Start a daicon file source actor.
///
/// A "source" returns a file from UUIDs. A "file source" uses a file as a source.
#[instrument("file-source-service", skip_all)]
pub fn start_file_source_service(
    ctx: &mut Context,
    file: Addr<FileMessage>,
) -> Result<Addr<SourceMessage>, Error> {
    let mut ctx = ctx.create()?;
    let addr = ctx.addr()?;

    let source = ctx.map(addr, Message::SourceMessage)?;

    // Start the root manager actor
    let actor = FileSourceService {
        write_header_result: ctx.map(addr, Message::WriteHeaderResult)?,
        file,
        table: None,

        get_tasks: Vec::new(),
        pending_slots: Vec::new(),
    };
    ctx.start(Options::default(), actor)?;

    // Immediately start table read, assuming at start for now
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Read {
            offset: 0,
            size: 64 * 1024,
            on_result: ctx.map_once(addr, Message::ReadTableResult)?,
        },
    };
    ctx.send(file, message);

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
        let mut data = Cursor::new(result.data);

        // Read the header
        let mut header = Header::default();
        data.read_exact(bytes_of_mut(&mut header))?;

        // TODO: Retry if the table's valid data is larger than what we've read, this can happen
        // sometimes

        // Read entries
        let mut entries = vec![Entry::default(); header.capacity() as usize];
        data.read_exact(cast_slice_mut(&mut entries))?;

        // Mark all valid entries as both valid and allocated
        let mut entries_meta = vec![EntryMeta::default(); entries.len()];
        for i in 0..header.valid() as usize {
            entries_meta[i].valid = true;
            entries_meta[i].allocated = true;
        }

        // Store the cached data
        let table = CachedTable {
            offset: 0,
            entries,
            entries_meta,
        };
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
    let offset = entry_offset(table.offset, index);
    ctx.send(on_result, offset);

    // Write the new header with the updated valid count
    // TODO: Wait until the task tells us to validate
    // TODO: Get the entry back from the task, currently the cache is wrong
    table.mark_valid(index, Entry::default());
    let header = table.create_header();
    let message = FileMessage {
        id: Uuid::new_v4(),
        action: FileAction::Write {
            location: WriteLocation::Offset(table.offset),
            data: bytes_of(&header).to_owned(),
            on_result: write_header_result,
        },
    };
    ctx.send(file, message);

    true
}

/// In-memory cached representation of a table.
struct CachedTable {
    offset: u64,
    entries: Vec<Entry>,
    entries_meta: Vec<EntryMeta>,
}

#[derive(Default, Clone)]
struct EntryMeta {
    valid: bool,
    allocated: bool,
}

impl CachedTable {
    fn find(&self, id: Uuid) -> Option<Entry> {
        self.entries.iter().find(|e| e.id() == id).cloned()
    }

    fn try_allocate(&mut self) -> Option<usize> {
        // Get a slot that hasn't been allocated yet
        let (index, meta) = self
            .entries_meta
            .iter_mut()
            .enumerate()
            .find(|(_, v)| !v.allocated)?;

        // Mark it as allocated
        meta.allocated = true;

        Some(index)
    }

    /// Mark that an entry is now available, with the given data.
    ///
    /// Eventually, this will result in the header's count being updated.
    fn mark_valid(&mut self, index: usize, entry: Entry) {
        self.entries[index] = entry;
        self.entries_meta[index].valid = true;
    }

    fn create_header(&self) -> Header {
        // Count the amount of entries until we hit one that isn't valid
        let mut valid = 0;
        for meta in &self.entries_meta {
            if !meta.valid {
                break;
            }

            valid += 1;
        }

        let mut header = Header::default();
        header.set_capacity(self.entries.len() as u16);
        header.set_valid(valid);
        header
    }
}

fn entry_offset(offset: u64, index: usize) -> u64 {
    offset + size_of::<Header>() as u64 + (size_of::<Entry>() as u64 * index as u64)
}
