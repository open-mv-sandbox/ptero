use anyhow::{Context, Error};
use clap::Args;
use ptero_daicon::{io::ReadWriteCmd, FileManagerCmd, FileManagerData};
use ptero_pack::{start_add_data, AddData};
use stewart::{
    utils::{ActorT, AddrT, SystemExt},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, Level};
use uuid::Uuid;

use crate::io::start_file_read_write;

/// Add files to a dacti package.
#[derive(Args, Debug)]
pub struct AddCommand {
    /// The path of the package to add files to.
    #[arg(short, long, value_name = "PATH")]
    package: String,

    /// The input location of the file to add.
    #[arg(short, long, value_name = "PATH")]
    input: String,

    /// The UUID to assign the input file.
    #[arg(short, long, value_name = "UUID")]
    uuid: Uuid,
}

pub fn start(system: &mut System, data: AddCommand) {
    system.start_with("ppcli-add", data, AddCommandActor::start);
}

struct AddCommandActor {
    message: Option<AddrT<FileManagerCmd>>,
    read_write: AddrT<ReadWriteCmd>,
    input: Vec<u8>,
    uuid: Uuid,
}

impl AddCommandActor {
    fn start(
        system: &mut System,
        addr: AddrT<AddrT<FileManagerCmd>>,
        data: AddCommand,
    ) -> Result<Self, Error> {
        event!(Level::INFO, "adding file to package");

        // Start the file actor
        let input = std::fs::read(&data.input)?;
        let read_write = start_file_read_write(system, data.package);

        // Start the file manager actor
        let file_manager = FileManagerData {
            on_ready: addr,
            read_write,
        };
        ptero_daicon::start_file_manager(system, file_manager);

        Ok(AddCommandActor {
            message: None,
            read_write,
            input,
            uuid: data.uuid,
        })
    }
}

impl ActorT for AddCommandActor {
    type Message = AddrT<FileManagerCmd>;

    fn reduce(&mut self, message: AddrT<FileManagerCmd>) -> Result<AfterReduce, Error> {
        self.message = Some(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        let message = self.message.take().context("incorrect state")?;

        let add_data = AddData {
            file: self.read_write,
            file_manager: message,
            data: self.input.clone(),
            uuid: self.uuid,
        };
        start_add_data(system, add_data);

        Ok(AfterProcess::Nothing)
    }
}
