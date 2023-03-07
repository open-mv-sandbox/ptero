use anyhow::Error;
use daicon::{ComponentEntry, ComponentTableHeader};
use ptero_io::ReadWriteCmd;
use stewart::{
    utils::{start_map, ActorT, AddrT},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, instrument, Level};
use uuid::Uuid;

use crate::{start_find_component, FindComponentData};

/// Start a daicon file manager.
#[instrument("file-manager", skip_all)]
pub fn start_file_manager(
    system: &mut System,
    read_write: AddrT<ReadWriteCmd>,
) -> Result<AddrT<FileManagerCmd>, Error> {
    let addr = system.create();

    let api_addr = start_map(system, |c| Message::Command(c), addr)?;

    let actor = FileManagerActor {
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
                        start_find_component(system, data)?;
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
