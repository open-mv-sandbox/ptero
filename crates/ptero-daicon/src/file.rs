use std::{
    collections::HashMap,
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon::{Entry, Header};
use ptero_file::{FileMessage, Operation, ReadResult, WriteLocation, WriteResult};
use stewart::{Actor, Addr, After, Id, Options, System};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub enum SourceMessage {
    /// Get the data associated with a UUID.
    /// TODO: Reply with an inner file actor addr instead.
    Get {
        id: Uuid,
        on_result: Addr<ReadResult>,
    },
    /// Set the data associated with a UUID.
    Set { id: Uuid, data: Vec<u8> },
}

/// Start a daicon file source actor.
///
/// A "source" returns a file from UUIDs. A "file source" uses a file as a source.
#[instrument("file-source", skip_all)]
pub fn start_file_source(
    system: &mut System,
    parent: Id,
    file: Addr<FileMessage>,
) -> Result<Addr<SourceMessage>, Error> {
    let (id, addr) = system.create(parent)?;

    let source = start_map(system, parent, addr, Message::Source)?;
    let read_result = start_map(system, parent, addr, Message::ReadResult)?;
    let write_result = start_map(system, parent, addr, Message::WriteResult)?;

    // Start the root manager actor
    let actor = FileSourceActor {
        write_result,
        file,
        table: None,

        pending_get: Vec::new(),
        pending_set: Vec::new(),
        pending_append: HashMap::new(),
    };
    system.start(id, Options::default(), actor)?;

    // Immediately start table read, assuming at start for now
    let message = FileMessage {
        id: Uuid::new_v4(),
        operation: Operation::Read {
            offset: 0,
            size: 64 * 1024,
            on_result: read_result,
        },
    };
    system.send(file, message);

    Ok(source)
}

struct FileSourceActor {
    write_result: Addr<WriteResult>,
    file: Addr<FileMessage>,
    table: Option<CachedTable>,

    pending_get: Vec<(Uuid, Addr<ReadResult>)>,
    pending_set: Vec<Entry>,
    pending_append: HashMap<Uuid, (Uuid, u64)>,
}

impl FileSourceActor {
    fn on_source_message(
        &mut self,
        system: &mut System,
        message: SourceMessage,
    ) -> Result<(), Error> {
        match message {
            SourceMessage::Get { id, on_result } => {
                event!(Level::INFO, ?id, "received get");
                self.pending_get.push((id, on_result));
            }
            SourceMessage::Set { id, data } => {
                event!(Level::INFO, ?id, bytes = data.len(), "received set");

                // Append the data to the file
                let size = data.len() as u64;
                let message = FileMessage {
                    id: Uuid::new_v4(),
                    operation: Operation::Write {
                        location: WriteLocation::Append,
                        data,
                        on_result: self.write_result,
                    },
                };
                self.pending_append.insert(message.id, (id, size));
                system.send(self.file, message);
            }
        }

        Ok(())
    }

    fn on_read_result(&mut self, result: ReadResult) -> Result<(), Error> {
        let mut data = Cursor::new(result.data);

        // Read the header
        let mut header = Header::default();
        data.read_exact(bytes_of_mut(&mut header))?;

        // TODO: Retry if the table is larger than what we've read, this can happen sometimes

        // Read entries
        let mut entries = vec![Entry::default(); header.valid() as usize];
        data.read_exact(cast_slice_mut(&mut entries))?;

        // Cache data
        let table = CachedTable {
            dirty: false,
            offset: 0,
            capacity: header.capacity(),
            entries,
        };
        self.table = Some(table);

        // TODO: Follow additional headers

        Ok(())
    }

    fn on_write_result(&mut self, result: WriteResult) -> Result<(), Error> {
        // Check if this write result is for a pending file append
        let (id, size) = if let Some(value) = self.pending_append.remove(&result.id) {
            value
        } else {
            return Ok(());
        };

        // Queue to add to a table
        let mut entry = Entry::default();
        entry.set_id(id);
        entry.set_offset(result.offset);
        entry.set_size(size);
        self.pending_set.push(entry);

        Ok(())
    }

    fn check_pending(&mut self, system: &mut System) {
        let table = if let Some(table) = self.table.as_mut() {
            table
        } else {
            return;
        };

        // Check all pending
        self.pending_get
            .retain(|(id, on_result)| !try_get(system, self.file, table, *id, *on_result));
        self.pending_set
            .retain(|entry| !try_set(system, self.write_result, self.file, table, *entry));

        // TODO: Reply failure if we ran out of tables to read, and couldn't find it
        // TODO: Allocate new tables if we ran out of free spaces
    }
}

impl Actor for FileSourceActor {
    type Message = Message;

    fn handle(&mut self, system: &mut System, message: Message) -> Result<After, Error> {
        match message {
            Message::Source(message) => {
                self.on_source_message(system, message)?;
            }
            Message::ReadResult(result) => {
                self.on_read_result(result)?;
            }
            Message::WriteResult(result) => {
                self.on_write_result(result)?;
            }
        };

        // Check if we can resolve any get requests
        self.check_pending(system);

        Ok(After::Nothing)
    }
}

enum Message {
    Source(SourceMessage),
    ReadResult(ReadResult),
    WriteResult(WriteResult),
}

fn try_get(
    system: &mut System,
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
        operation: Operation::Read {
            offset: entry.offset(),
            size: entry.size(),
            on_result,
        },
    };
    system.send(file, message);

    true
}

fn try_set(
    system: &mut System,
    write_result: Addr<WriteResult>,
    file: Addr<FileMessage>,
    table: &mut CachedTable,
    entry: Entry,
) -> bool {
    let index = if let Some(index) = table.try_add(entry) {
        index
    } else {
        return false;
    };

    event!(
        Level::INFO,
        id = ?entry.id(),
        "set entry"
    );

    // We succeeded, write the new entry
    let entry_offset = entry_offset(table.offset, index);
    let message = FileMessage {
        id: Uuid::new_v4(),
        operation: Operation::Write {
            location: WriteLocation::Offset(entry_offset),
            data: bytes_of(&entry).to_owned(),
            on_result: write_result,
        },
    };
    system.send(file, message);

    // Write the header
    let header = table.create_header();
    let message = FileMessage {
        id: Uuid::new_v4(),
        operation: Operation::Write {
            location: WriteLocation::Offset(table.offset),
            data: bytes_of(&header).to_owned(),
            on_result: write_result,
        },
    };
    system.send(file, message);

    true
}

/// In-memory cached representation of a table.
struct CachedTable {
    dirty: bool,
    offset: u64,
    capacity: u16,
    entries: Vec<Entry>,
}

impl CachedTable {
    fn find(&self, id: Uuid) -> Option<Entry> {
        self.entries.iter().find(|e| e.id() == id).cloned()
    }

    fn try_add(&mut self, entry: Entry) -> Option<usize> {
        if self.entries.len() >= self.capacity as usize {
            return None;
        }

        self.entries.push(entry);
        self.dirty = true;

        Some(self.entries.len() - 1)
    }

    fn create_header(&self) -> Header {
        let mut header = Header::default();
        header.set_capacity(self.capacity);
        header.set_valid(self.entries.len() as u16);
        header
    }
}

fn entry_offset(offset: u64, index: usize) -> u64 {
    offset + size_of::<Header>() as u64 + (size_of::<Entry>() as u64 * index as u64)
}
