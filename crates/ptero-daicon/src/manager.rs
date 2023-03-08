use anyhow::Error;
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::ReadWriteCmd;
use stewart::{ActorT, AddrT, AfterProcess, AfterReduce, Id, Info, System};
use stewart_utils::start_map_t;
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{start_find_component, FindComponentData};

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    parent: Id,
    read_write: AddrT<ReadWriteCmd>,
) -> Result<AddrT<FileManagerCmd>, Error> {
    let info = system.create_actor(parent)?;

    let api_addr = start_map_t(system, info.id(), Message::Command, info.addr())?;

    let actor = FileManagerActor {
        info,
        read_write,
        queue: Vec::new(),
    };
    system.start_actor(info, actor)?;

    Ok(api_addr)
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
    info: Info<Self>,
    read_write: AddrT<ReadWriteCmd>,
    queue: Vec<Message>,
}

impl ActorT for FileManagerActor {
    type Message = Message;

    fn reduce(&mut self, _system: &mut System, message: Message) -> Result<AfterReduce, Error> {
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
                        let data = FindComponentData {
                            target: id,
                            package: self.read_write,
                            reply: on_result,
                        };
                        start_find_component(system, self.info.id(), data)?;
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
