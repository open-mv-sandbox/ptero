use anyhow::Error;
use clap::Args;
use ptero_pack::AddData;
use stewart::{Actor, After, Options, System};
use tracing::{event, instrument, Level};
use uuid::Uuid;

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

#[instrument("add-command", skip_all)]
pub fn start(system: &mut System, data: AddCommand) -> Result<(), Error> {
    event!(Level::INFO, "adding file to package");

    let info = system.create(system.root_id())?;

    let input = std::fs::read(&data.input)?;

    // Start managers for the package
    let read_write = ptero_io::start_file_read_write(system, info.id(), data.package, false)?;
    let file_manager = ptero_daicon::start_file_manager(system, info.id(), read_write)?;

    // Start the add data command
    let add_data = AddData {
        file: read_write,
        file_manager,
        data: input,
        uuid: data.uuid,
    };
    ptero_pack::start_add_data(system, info.id(), add_data)?;

    system.start(info, Options::default(), AddCommandActor)?;

    Ok(())
}

struct AddCommandActor;

impl Actor for AddCommandActor {
    type Message = ();

    fn handle(&mut self, _system: &mut System, _message: ()) -> Result<After, Error> {
        unimplemented!()
    }
}
