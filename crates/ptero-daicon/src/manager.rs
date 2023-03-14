use std::mem::size_of;

use anyhow::{Context, Error};
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::ReadWriteCmd;
use stewart::{Actor, Addr, After, Id, Info, Options, System};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::read::{start_read_entries, start_read_header};

pub enum FileManagerCommand {
    GetComponent(GetComponentCommand),
}

pub struct GetComponentCommand {
    pub id: Uuid,
    pub on_result: Addr<GetComponentResult>,
}

pub struct GetComponentResult {
    pub offset: u64,
    pub entry: ComponentEntry,
}

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    parent: Id,
    read_write: Addr<ReadWriteCmd>,
) -> Result<Addr<FileManagerCommand>, Error> {
    let info = system.create(parent)?;

    let command = start_map(system, parent, info.addr(), FileManagerMessage::Command)?;

    // Start the root manager actor
    let actor = FileManagerActor {
        info,
        read_write,

        pending: Vec::new(),
        header: None,
        entries: None,
    };
    system.start(info, Options::default(), actor)?;

    // Immediately start reading the first header
    start_read_header(system, info.id(), read_write, info.addr())?;

    Ok(command)
}

struct FileManagerActor {
    info: Info<Self>,
    read_write: Addr<ReadWriteCmd>,

    pending: Vec<GetComponentCommand>,
    header: Option<ComponentTableHeader>,
    entries: Option<Vec<ComponentEntry>>,
}

impl FileManagerActor {
    fn on_command(&mut self, command: FileManagerCommand) -> Result<(), Error> {
        match command {
            FileManagerCommand::GetComponent(command) => {
                event!(Level::INFO, id = ?command.id, "queuing get-component");
                self.pending.push(command);
            }
        }

        Ok(())
    }

    fn on_entries_changed(&mut self, system: &mut System) -> Result<(), Error> {
        // We will have more tables to check later, but for now just the one
        let entries = self.entries.as_ref().context("invalid state")?;

        self.pending.retain(|pending| {
            for entry in entries {
                if entry.type_id() != pending.id {
                    continue;
                }

                // We found a matching component!
                event!(
                    Level::INFO,
                    id = ?entry.type_id(),
                    "sending found component back"
                );
                let header = self.header.unwrap();
                let result = GetComponentResult {
                    offset: header.entries_offset(),
                    entry: *entry,
                };
                system.send(pending.on_result, result);
                return false;
            }

            // TODO: Failure if we ran out and couldn't find it

            true
        });

        Ok(())
    }
}

impl Actor for FileManagerActor {
    type Message = FileManagerMessage;

    fn handle(&mut self, system: &mut System, message: FileManagerMessage) -> Result<After, Error> {
        let mut entries_changed = false;

        match message {
            FileManagerMessage::Command(command) => self.on_command(command)?,
            FileManagerMessage::Header(header) => {
                event!(Level::DEBUG, "caching header");
                self.header = Some(header);

                // Start reading the entries for this header
                start_read_entries(
                    system,
                    self.info.id(),
                    self.read_write,
                    8 + size_of::<ComponentTableHeader>() as u64,
                    header.length() as usize,
                    self.info.addr(),
                )?;

                // TODO: Follow additional headers
            }
            FileManagerMessage::Entries(entries) => {
                event!(Level::DEBUG, length = entries.len(), "caching entries");
                self.entries = Some(entries);
                entries_changed = true;
            }
        };

        // Check if we can resolve any pending requests
        if entries_changed {
            self.on_entries_changed(system)?;
        }

        Ok(After::Nothing)
    }
}

pub enum FileManagerMessage {
    Command(FileManagerCommand),
    Header(ComponentTableHeader),
    Entries(Vec<ComponentEntry>),
}
