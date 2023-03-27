use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::Error;
use bytemuck::{bytes_of, bytes_of_mut, cast_slice_mut};
use daicon::{Entry, Header};
use ptero_file::FileMessage;
use stewart::{Actor, Addr, After, Id, Options, System};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};
use uuid::Uuid;

pub enum SourceMessage {
    /// Get the data associated with a UUID.
    /// TODO: Reply with an inner file actor addr instead.
    Get { id: Uuid, on_result: Addr<Vec<u8>> },
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
    let info = system.create(parent)?;

    let command = start_map(system, parent, info.addr(), Message::Source)?;
    let read_table_result = start_map(system, parent, info.addr(), Message::ReadTableResult)?;

    // Start the root manager actor
    let actor = FileSourceActor {
        file,

        table: None,
        // TODO: Move this to the file handler with an "append" message, replying the location.
        append_offset: 64 * 1024,

        pending_get: Vec::new(),
        pending_set: Vec::new(),
    };
    system.start(info, Options::default(), actor)?;

    // Immediately start table read, assuming at start for now
    let message = FileMessage::Read {
        offset: 0,
        size: 64 * 1024,
        on_result: read_table_result,
    };
    system.send(file, message);

    Ok(command)
}

struct FileSourceActor {
    file: Addr<FileMessage>,

    table: Option<CachedTable>,
    append_offset: u64,

    pending_get: Vec<(Uuid, Addr<Vec<u8>>)>,
    pending_set: Vec<Entry>,
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

                // Append to the file
                let offset = self.append_offset;
                let size = data.len() as u64;
                self.append_offset = self.append_offset + size;
                let message = FileMessage::Write { offset, data };
                system.send(self.file, message);

                // Queue to add to a table
                let mut entry = Entry::default();
                entry.set_id(id);
                entry.set_offset(offset);
                entry.set_size(size);
                self.pending_set.push(entry);
            }
        }

        Ok(())
    }

    fn on_read_table(&mut self, data: Vec<u8>) -> Result<(), Error> {
        let mut data = Cursor::new(data);

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
            .retain(|entry| !try_set(system, self.file, table, *entry));

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
            Message::ReadTableResult(data) => {
                self.on_read_table(data)?;
            }
        };

        // Check if we can resolve any get requests
        self.check_pending(system);

        Ok(After::Nothing)
    }
}

enum Message {
    Source(SourceMessage),
    ReadTableResult(Vec<u8>),
}

fn try_get(
    system: &mut System,
    file: Addr<FileMessage>,
    table: &mut CachedTable,
    id: Uuid,
    on_result: Addr<Vec<u8>>,
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
    let message = FileMessage::Read {
        offset: entry.offset(),
        size: entry.size(),
        on_result,
    };
    system.send(file, message);

    true
}

fn try_set(
    system: &mut System,
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
    let message = FileMessage::Write {
        offset: entry_offset,
        data: bytes_of(&entry).to_owned(),
    };
    system.send(file, message);

    // Write the header
    let header = table.create_header();
    let message = FileMessage::Write {
        offset: table.offset,
        data: bytes_of(&header).to_owned(),
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
