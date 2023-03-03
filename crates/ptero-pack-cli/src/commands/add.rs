use anyhow::{Context, Error};
use clap::Args;
use ptero_daicon::io::ReadWrite;
use ptero_pack::AddData;
use stewart::{Actor, ActorAddr, AfterProcess, AfterReduce, Factory, System};
use tracing::{event, Level};
use uuid::Uuid;

use crate::io::FileReadWrite;

/// Add files to a dacti package.
#[derive(Factory, Args, Debug)]
#[factory(AddCommandActor::start)]
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

struct AddCommandActor {
    package: Option<ActorAddr<ReadWrite>>,
    input: Vec<u8>,
    uuid: Uuid,
}

impl AddCommandActor {
    pub fn start(
        system: &mut System,
        addr: ActorAddr<ActorAddr<ReadWrite>>,
        data: AddCommand,
    ) -> Result<Self, Error> {
        event!(Level::INFO, "adding file to package");

        let input = std::fs::read(&data.input)?;

        let start_file = FileReadWrite {
            path: data.package,
            reply: addr,
        };
        system.start(start_file);

        Ok(AddCommandActor {
            package: None,
            input,
            uuid: data.uuid,
        })
    }
}

impl Actor for AddCommandActor {
    type Protocol = ActorAddr<ReadWrite>;

    fn reduce(&mut self, message: ActorAddr<ReadWrite>) -> Result<AfterReduce, Error> {
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
        system.start(add_data);

        Ok(AfterProcess::Nothing)
    }
}
