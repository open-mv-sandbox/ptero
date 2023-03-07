use anyhow::Error;
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::ReadWriteCmd;
use stewart::{
    utils::{start_map, ActorT, AddrT},
    ActorId, AfterProcess, AfterReduce, System,
};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{start_find_component, FindComponentData};

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    parent: ActorId,
    read_write: AddrT<ReadWriteCmd>,
) -> Result<AddrT<FileManagerCmd>, Error> {
    let (id, addr) = system.create_addr(parent)?;

    let api_addr = start_map(system, id, |c| Message::Command(c), addr)?;

    let actor = FileManagerActor {
        id,
        read_write,
        queue: Vec::new(),
    };
    system.start(addr, actor)?;

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
    id: ActorId,
    read_write: AddrT<ReadWriteCmd>,
    queue: Vec<Message>,
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
                        let data = FindComponentData {
                            target: id,
                            package: self.read_write,
                            reply: on_result,
                        };
                        start_find_component(system, self.id, data)?;
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
