use anyhow::Error;
use clap::Args;
use ptero_daicon::{SourceAction, SourceMessage};
use stewart::{Actor, After, Context, Options};
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

    /// Path of the input file to read.
    #[arg(short, long, value_name = "PATH")]
    input: String,
}

#[instrument("add-command", skip_all)]
pub fn start(ctx: &mut Context, command: SetCommand) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let mut ctx = ctx.create()?;

    let data = std::fs::read(&command.input)?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::start_system_file(&mut ctx, &command.target, false)?;
    let source = ptero_daicon::start_file_source_service(&mut ctx, file)?;

    // Add the data to the source
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Set {
            id: command.id,
            data,
            on_result: ctx.addr()?,
        },
    };
    ctx.send(source, message);

    ctx.start(Options::default(), AddCommandActor)?;

    Ok(())
}

struct AddCommandActor;

impl Actor for AddCommandActor {
    type Message = ();

    fn handle(&mut self, _ctx: &mut Context, _message: ()) -> Result<After, Error> {
        Ok(After::Stop)
    }
}
