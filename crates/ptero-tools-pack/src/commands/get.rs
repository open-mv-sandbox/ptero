use anyhow::Error;
use clap::Args;
use ptero_daicon::SourceMessage;
use ptero_file::ReadResult;
use stewart::{Actor, After, Options, System};
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// Get an entry from a daicon file.
#[derive(Args, Debug)]
pub struct GetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// UUID of the entry to get.
    #[arg(short = 'd', long, value_name = "UUID")]
    id: Uuid,

    /// Path of the output file to write.
    #[arg(short, long, value_name = "PATH")]
    output: String,
}

#[instrument("add-command", skip_all)]
pub fn start(system: &mut System, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let (id, addr) = system.create_root()?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::start_system_file(system, id, &command.target, false)?;
    let source = ptero_daicon::start_file_source(system, id, file)?;

    // Add the data to the source
    let message = SourceMessage::Get {
        id: command.id,
        on_result: addr,
    };
    system.send(source, message);

    let actor = GetCommandActor {
        output: command.output,
    };
    system.start(id, Options::default(), actor)?;

    Ok(())
}

struct GetCommandActor {
    output: String,
}

impl Actor for GetCommandActor {
    type Message = ReadResult;

    fn handle(&mut self, _system: &mut System, message: ReadResult) -> Result<After, Error> {
        std::fs::write(&self.output, message.data)?;
        Ok(After::Stop)
    }
}
