use std::mem::size_of;

use anyhow::Error;
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::{Read, Write};
use stewart::{Actor, Addr, After, Id, Info, Options, System};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::read::{start_read_entries, start_read_header};

pub enum FileManagerCommand {
    /// Load a component, reading tables until it's found.
    Load(Load),
    /// Overwrite or add a component in a table.
    Store(Store),
    /// Flush pending changes to the target writer.
    Flush(Addr<Write>),
}

pub struct Load {
    pub id: Uuid,
    pub on_result: Addr<(ComponentTableHeader, ComponentEntry)>,
}

pub struct Store {
    pub id: Uuid,
    pub entry: ComponentEntry,
}

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    parent: Id,
    read: Addr<Read>,
) -> Result<Addr<FileManagerCommand>, Error> {
    let info = system.create(parent)?;

    let command = start_map(system, parent, info.addr(), Message::Command)?;

    // Start the root manager actor
    let actor = FileManagerActor {
        info,
        read,

        table: CachedTable::default(),
        pending: Vec::new(),
    };
    system.start(info, Options::default(), actor)?;

    // Immediately start reading the first header
    start_read_header(system, info.id(), read, info.addr())?;

    Ok(command)
}

struct FileManagerActor {
    info: Info<Self>,
    read: Addr<Read>,

    table: CachedTable,
    pending: Vec<Load>,
}

impl FileManagerActor {
    fn on_command(&mut self, command: FileManagerCommand) -> Result<(), Error> {
        match command {
            FileManagerCommand::Load(command) => {
                event!(Level::INFO, id = ?command.id, "queuing load");
                self.pending.push(command);
            }
            FileManagerCommand::Store(command) => {
                event!(Level::INFO, id = ?command.id, "queuing store");
                // TODO
            }
            FileManagerCommand::Flush(_write) => {
                event!(Level::INFO, "flushing changes");
                // TODO
            }
        }

        Ok(())
    }

    fn process(&mut self, system: &mut System) {
        self.pending.retain(|pending| {
            let entry = if let Some(entry) = self.table.entry(pending.id) {
                entry
            } else {
                return true;
            };

            // We found a matching component!
            event!(
                Level::INFO,
                id = ?entry.type_id(),
                "sending found component back"
            );
            system.send(pending.on_result, (self.table.header, entry));

            false
        });

        // TODO: Reply failure if we ran out of tables to read, and couldn't find it
    }
}

impl Actor for FileManagerActor {
    type Message = Message;

    fn handle(&mut self, system: &mut System, message: Message) -> Result<After, Error> {
        let mut process = false;

        match message {
            Message::Command(command) => {
                self.on_command(command)?;
                process = true;
            }
            Message::Header(header) => {
                event!(Level::DEBUG, "caching header");
                self.table.header = header;

                // Start reading the entries for this header
                start_read_entries(
                    system,
                    self.info.id(),
                    self.read,
                    8 + size_of::<ComponentTableHeader>() as u64,
                    header.length() as usize,
                    self.info.addr(),
                )?;

                // TODO: Follow additional headers
            }
            Message::Entries(entries) => {
                event!(Level::DEBUG, length = entries.len(), "caching entries");
                self.table.entries = entries;
                process = true;
            }
        };

        // Check if we can resolve any pending requests
        if process {
            self.process(system);
        }

        Ok(After::Nothing)
    }
}

pub enum Message {
    Command(FileManagerCommand),
    Header(ComponentTableHeader),
    Entries(Vec<ComponentEntry>),
}

#[derive(Default)]
struct CachedTable {
    header: ComponentTableHeader,
    entries: Vec<ComponentEntry>,
}

impl CachedTable {
    fn entry(&self, id: Uuid) -> Option<ComponentEntry> {
        self.entries.iter().find(|e| e.type_id() == id).cloned()
    }
}
