use anyhow::Error;
use clap::Args;
use ptero_pack::{start_add_data, AddData};
use stewart::{utils::ActorT, AfterProcess, AfterReduce, System};
use tracing::{event, Level};
use uuid::Uuid;

use ptero_io::start_file_read_write;

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

pub fn start(system: &mut System, data: AddCommand) -> Result<(), Error> {
    AddCommandActor::start(system, data)
}

struct AddCommandActor {}

impl AddCommandActor {
    fn start(system: &mut System, data: AddCommand) -> Result<(), Error> {
        event!(Level::INFO, "adding file to package");

        let addr = system.create("ppcli-add");

        // Start the file actor
        let input = std::fs::read(&data.input)?;
        let read_write = start_file_read_write(system, data.package)?;

        // Start the file manager actor
        let file_manager = ptero_daicon::start_file_manager(system, read_write)?;

        // Start the add data command
        let add_data = AddData {
            file: read_write,
            file_manager,
            data: input,
            uuid: data.uuid,
        };
        start_add_data(system, add_data)?;

        system.start(addr, Self {})?;

        Ok(())
    }
}

impl ActorT for AddCommandActor {
    type Message = ();

    fn reduce(&mut self, _message: ()) -> Result<AfterReduce, Error> {
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        // TODO: Handle success/failure
        Ok(AfterProcess::Nothing)
    }
}
