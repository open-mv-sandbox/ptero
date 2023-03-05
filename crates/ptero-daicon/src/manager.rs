use anyhow::Error;
use stewart::{
    utils::{ActorAddrT, ActorT, SystemExt},
    AfterProcess, AfterReduce, System,
};

/// Start a daicon file manager.
pub fn start_file_manager(system: &mut System, data: FileManagerData) {
    system.start_with("pd-file-manager", data, FileManagerActor::start);
}

pub struct FileManagerData {
    pub on_ready: ActorAddrT<ActorAddrT<FileOp>>,
}

pub enum FileOp {}

struct FileManagerActor {}

impl FileManagerActor {
    fn start(
        system: &mut System,
        addr: ActorAddrT<FileOp>,
        data: FileManagerData,
    ) -> Result<Self, anyhow::Error> {
        system.handle(data.on_ready, addr);

        Ok(FileManagerActor {})
    }
}

impl ActorT for FileManagerActor {
    type Message = FileOp;

    fn reduce(&mut self, _message: FileOp) -> Result<AfterReduce, Error> {
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        Ok(AfterProcess::Nothing)
    }
}
