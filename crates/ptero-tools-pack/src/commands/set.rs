use anyhow::Error;
use clap::Args;
use ptero_daicon::SourceMessage;
use stewart::{Actor, After, Options, System};
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// Set or add an entry in a daicon file.
#[derive(Args, Debug)]
pub struct SetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// UUID to assign the added data.
    #[arg(short = 'd', long, value_name = "UUID")]
    id: Uuid,

    /// Path of the input data.
    #[arg(short, long, value_name = "PATH")]
    input: String,
}

#[instrument("add-command", skip_all)]
pub fn start(system: &mut System, command: SetCommand) -> Result<(), Error> {
    event!(Level::INFO, "adding file to package");

    let info = system.create_root()?;

    let data = std::fs::read(&command.input)?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::start_system_file(system, info.id(), &command.target, false)?;
    let source = ptero_daicon::start_file_source(system, info.id(), file)?;

    // Add the data to the source
    let message = SourceMessage::Set {
        id: command.id,
        data,
    };
    system.send(source, message);

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
