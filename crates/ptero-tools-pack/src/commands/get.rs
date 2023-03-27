use anyhow::Error;
use clap::Args;
use ptero_daicon::SourceMessage;
use stewart::{Actor, After, Options, System};
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// Get an entry from a daicon file.
#[derive(Args, Debug)]
pub struct GetCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,

    /// The UUID of the entry to get.
    #[arg(short = 'd', long, value_name = "UUID")]
    id: Uuid,

    /// Path of the output file to write.
    #[arg(short, long, value_name = "PATH")]
    output: String,
}

#[instrument("add-command", skip_all)]
pub fn start(system: &mut System, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "adding file to package");

    let info = system.create_root()?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::start_system_file(system, info.id(), &command.target, false)?;
    let source = ptero_daicon::start_file_source(system, info.id(), file)?;

    // Add the data to the source
    let message = SourceMessage::Get {
        id: command.id,
        on_result: info.addr(),
    };
    system.send(source, message);

    let actor = GetCommandActor {
        output: command.output,
    };
    system.start(info, Options::default(), actor)?;

    Ok(())
}

struct GetCommandActor {
    output: String,
}

impl Actor for GetCommandActor {
    type Message = Vec<u8>;

    fn handle(&mut self, _system: &mut System, message: Vec<u8>) -> Result<After, Error> {
        std::fs::write(&self.output, message)?;
        Ok(After::Stop)
    }
}
