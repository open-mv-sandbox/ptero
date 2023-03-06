use anyhow::Error;
use daicon::{ComponentEntry, ComponentTableHeader};
use stewart::{
    utils::{start_map, ActorT, AddrT, SystemExt},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, Level};
use uuid::Uuid;

use crate::{io::ReadWriteCmd, start_find_component, FindComponent};

/// Start a daicon file manager.
pub fn start_file_manager(system: &mut System, data: FileManagerData) {
    system.start_with("pd-file-manager", data, FileManagerActor::start);
}

pub struct FileManagerData {
    pub on_ready: AddrT<AddrT<FileManagerCmd>>,
    pub read_write: AddrT<ReadWriteCmd>,
}

pub enum FileManagerCmd {
    GetComponent {
        id: Uuid,
        on_result: AddrT<FindComponentResult>,
    },
}

pub struct FindComponentResult {
    pub header: ComponentTableHeader,
    pub entry: ComponentEntry,
}

/// Root manager actor.
struct FileManagerActor {
    read_write: AddrT<ReadWriteCmd>,
    queue: Vec<Message>,
}

impl FileManagerActor {
    fn start(
        system: &mut System,
        addr: AddrT<Message>,
        data: FileManagerData,
    ) -> Result<Self, anyhow::Error> {
        let api_addr = start_map(system, |c| Message::Command(c), addr);
        system.handle(data.on_ready, api_addr)?;

        Ok(FileManagerActor {
            read_write: data.read_write,
            queue: Vec::new(),
        })
    }
}

impl ActorT for FileManagerActor {
    type Message = Message;

    fn reduce(&mut self, message: Message) -> Result<AfterReduce, Error> {
        self.queue.push(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        for message in self.queue.drain(..) {
            match message {
                Message::Command(cmd) => match cmd {
                    FileManagerCmd::GetComponent { id, on_result } => {
                        event!(Level::INFO, "processing get-component");
                        // TODO: Clean this up
                        let data = FindComponent {
                            target: id,
                            package: self.read_write,
                            reply: on_result,
                        };
                        start_find_component(system, data);
                    }
                },
            }
        }

        Ok(AfterProcess::Nothing)
    }
}

enum Message {
    Command(FileManagerCmd),
}