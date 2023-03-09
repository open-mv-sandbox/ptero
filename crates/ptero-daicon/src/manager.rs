use std::{collections::VecDeque, mem::size_of};

use anyhow::{Context, Error};
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::ReadWriteCmd;
use stewart::{
    handler::{HandlerT, SenderT},
    schedule::{Process, Schedule},
    After, Id, Info, System,
};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::read::{start_read_entries, start_read_header};

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    parent: Id,
    schedule: Schedule,
    read_write: SenderT<ReadWriteCmd>,
) -> Result<SenderT<FileManagerCommand>, Error> {
    let info = system.create_actor(parent)?;

    // Public API mapping actor
    let api_addr = start_map(
        system,
        info.id(),
        SenderT::new(info),
        FileManagerMsg::Command,
    )?;

    // Start the root manager actor
    let actor = FileManagerActor {
        info,
        schedule,
        queue: VecDeque::new(),
        read_write,

        pending: Vec::new(),
        header: None,
        entries: None,
    };
    system.start_actor(info, actor)?;

    // Immediately start reading the first header
    start_read_header(system, info.id(), read_write, SenderT::new(info))?;

    Ok(api_addr)
}

pub enum FileManagerCommand {
    GetComponent(GetComponentCommand),
}

pub struct GetComponentCommand {
    pub id: Uuid,
    pub on_result: SenderT<GetComponentResult>,
}

pub struct GetComponentResult {
    pub offset: u64,
    pub entry: ComponentEntry,
}

struct FileManagerActor {
    info: Info<Self>,
    schedule: Schedule,
    queue: VecDeque<FileManagerMsg>,
    read_write: SenderT<ReadWriteCmd>,

    pending: Vec<GetComponentCommand>,
    header: Option<ComponentTableHeader>,
    entries: Option<Vec<ComponentEntry>>,
}

impl FileManagerActor {
    fn on_message(
        &mut self,
        system: &mut System,
        message: FileManagerMsg,
        entries_changed: &mut bool,
    ) -> Result<(), Error> {
        match message {
            FileManagerMsg::Command(command) => self.on_command(command)?,
            FileManagerMsg::Header(header) => {
                event!(Level::DEBUG, "caching header");
                self.header = Some(header);

                // Start reading the entries for this header
                start_read_entries(
                    system,
                    self.info.id(),
                    self.read_write,
                    8 + size_of::<ComponentTableHeader>() as u64,
                    header.length() as usize,
                    SenderT::new(self.info),
                )?;

                // TODO: Follow additional headers
            }
            FileManagerMsg::Entries(entries) => {
                event!(Level::DEBUG, length = entries.len(), "caching entries");
                self.entries = Some(entries);
                *entries_changed = true;
            }
        };

        Ok(())
    }

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
                    Level::DEBUG,
                    id = ?entry.type_id(),
                    "sending found component back"
                );
                let header = self.header.unwrap();
                let result = GetComponentResult {
                    offset: header.entries_offset(),
                    entry: *entry,
                };
                pending.on_result.send(system, result);
                return false;
            }

            // TODO: Failure if we ran out and couldn't find it

            true
        });

        Ok(())
    }
}

impl HandlerT for FileManagerActor {
    type Message = FileManagerMsg;

    fn handle(&mut self, _system: &mut System, message: FileManagerMsg) -> Result<After, Error> {
        self.queue.push_back(message);
        self.schedule.push(self.info)?;
        Ok(After::Nothing)
    }
}

impl Process for FileManagerActor {
    fn process(&mut self, system: &mut System) -> Result<After, Error> {
        let mut entries_changed = false;

        // Handle pending messages
        while let Some(message) = self.queue.pop_front() {
            self.on_message(system, message, &mut entries_changed)?;
        }

        // Check if we can resolve any pending requests
        if entries_changed {
            self.on_entries_changed(system)?;
        }

        Ok(After::Nothing)
    }
}

pub enum FileManagerMsg {
    Command(FileManagerCommand),
    Header(ComponentTableHeader),
    Entries(Vec<ComponentEntry>),
}
