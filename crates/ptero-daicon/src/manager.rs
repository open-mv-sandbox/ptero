use std::{collections::VecDeque, mem::size_of};

use anyhow::Error;
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::ReadWriteCmd;
use stewart::{ActorT, AddrT, AfterProcess, AfterReduce, Id, Info, System};
use stewart_utils::start_map;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::read::{start_read_entries, start_read_header};

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    parent: Id,
    read_write: AddrT<ReadWriteCmd>,
) -> Result<AddrT<FileManagerCmd>, Error> {
    let info = system.create_actor(parent)?;

    // Public API mapping actor
    let api_addr = start_map(system, info.id(), info.addr(), ManagerMsg::Command)?;

    // Start the root manager actor
    let actor = FileManagerActor::new(info, read_write);
    system.start_actor(info, actor)?;

    // Immediately start reading the first header
    start_read_header(system, info.id(), read_write, info.addr())?;

    Ok(api_addr)
}

pub enum FileManagerCmd {
    GetComponent(GetComponentCmd),
}

pub struct GetComponentCmd {
    pub id: Uuid,
    pub on_result: AddrT<GetComponentResult>,
}

pub struct GetComponentResult {
    pub header: ComponentTableHeader,
    pub entry: ComponentEntry,
}

struct FileManagerActor {
    info: Info<Self>,
    queue: VecDeque<ManagerMsg>,
    read_write: AddrT<ReadWriteCmd>,

    pending: Vec<GetComponentCmd>,
    header: Option<ComponentTableHeader>,
    entries: Option<Vec<ComponentEntry>>,
    waiting_for_read: bool,
}

impl FileManagerActor {
    fn new(info: Info<Self>, read_write: AddrT<ReadWriteCmd>) -> Self {
        Self {
            info,
            queue: VecDeque::new(),
            read_write,

            pending: Vec::new(),
            header: None,
            entries: None,
            waiting_for_read: true,
        }
    }

    fn on_command(&mut self, command: FileManagerCmd) -> Result<(), Error> {
        match command {
            FileManagerCmd::GetComponent(command) => {
                event!(Level::INFO, "processing get-component");
                self.pending.push(command);
            }
        }

        Ok(())
    }
}

impl ActorT for FileManagerActor {
    type Message = ManagerMsg;

    fn reduce(&mut self, _system: &mut System, message: ManagerMsg) -> Result<AfterReduce, Error> {
        self.queue.push_back(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        // Handle pending messages
        while let Some(message) = self.queue.pop_front() {
            match message {
                ManagerMsg::Command(command) => self.on_command(command)?,
                ManagerMsg::Header(header) => {
                    event!(Level::DEBUG, "caching header");
                    self.header = Some(header);
                    self.waiting_for_read = false;
                    // TODO: Follow additional headers
                }
                ManagerMsg::Entries(entries) => {
                    event!(Level::DEBUG, count = entries.len(), "caching entries");
                    self.entries = Some(entries);
                    self.waiting_for_read = false;
                }
            }
        }

        // Check if we need to do anything new
        if !self.pending.is_empty() && !self.waiting_for_read {
            if let Some(entries) = &self.entries {
                for pending in self.pending.drain(..) {
                    for entry in entries {
                        if entry.type_id() != pending.id {
                            continue;
                        }

                        event!(Level::DEBUG, "sending found component back");
                        let header = self.header.unwrap();
                        let result = GetComponentResult {
                            header,
                            entry: *entry,
                        };
                        system.handle(pending.on_result, result);
                    }

                    // TODO: Failure if we ran out and couldn't find it
                }
            } else {
                let header = self.header.as_ref().unwrap();
                start_read_entries(
                    system,
                    self.info.id(),
                    self.read_write,
                    8 + size_of::<ComponentTableHeader>() as u64,
                    header.length() as usize,
                    self.info.addr(),
                )?;
                self.waiting_for_read = true;
            }
        }

        Ok(AfterProcess::Nothing)
    }
}

pub enum ManagerMsg {
    Command(FileManagerCmd),
    Header(ComponentTableHeader),
    Entries(Vec<ComponentEntry>),
}
