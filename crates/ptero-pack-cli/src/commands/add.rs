use anyhow::{Context, Error};
use clap::Args;
use ptero_daicon::io::ReadWrite;
use ptero_pack::{start_add_data, AddData};
use stewart::{
    utils::{ActorAddrT, ActorT, SystemExt},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, Level};
use uuid::Uuid;

use crate::io::{start_read_write_file, FileReadWrite};

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
    package: Option<ActorAddrT<ReadWrite>>,
    input: Vec<u8>,
    uuid: Uuid,
}

impl AddCommandActor {
    fn start(
        system: &mut System,
        addr: ActorAddrT<ActorAddrT<ReadWrite>>,
        data: AddCommand,
    ) -> Result<Self, Error> {
        event!(Level::INFO, "adding file to package");

        let input = std::fs::read(&data.input)?;

        let start_file = FileReadWrite {
            path: data.package,
            reply: addr,
        };
        start_read_write_file(system, start_file);

        Ok(AddCommandActor {
            package: None,
            input,
            uuid: data.uuid,
        })
    }
}

impl ActorT for AddCommandActor {
    type Message = ActorAddrT<ReadWrite>;

    fn reduce(&mut self, message: ActorAddrT<ReadWrite>) -> Result<AfterReduce, Error> {
        self.package = Some(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        let package = self.package.take().context("incorrect state")?;

        let (input, uuid) = (self.input.clone(), self.uuid);
        let add_data = AddData {
            package,
            data: input,
            uuid,
        };
        start_add_data(system, add_data);

        Ok(AfterProcess::Nothing)
    }
}
