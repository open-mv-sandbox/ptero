use anyhow::Error;
use clap::Args;
use ptero_daicon::{SourceAction, SourceMessage};
use ptero_file::ReadResult;
use stewart::{Actor, After, Context, Options};
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
pub fn start(ctx: &mut Context, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let mut ctx = ctx.create()?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::start_system_file(&mut ctx, &command.target, false)?;
    let source = ptero_daicon::start_file_source_service(&mut ctx, file)?;

    // Add the data to the source
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get {
            id: command.id,
            on_result: ctx.addr()?,
        },
    };
    ctx.send(source, message);

    let actor = GetCommandActor {
        output: command.output,
    };
    ctx.start(Options::default(), actor)?;

    Ok(())
}

struct GetCommandActor {
    output: String,
}

impl Actor for GetCommandActor {
    type Message = ReadResult;

    fn handle(&mut self, _ctx: &mut Context, message: ReadResult) -> Result<After, Error> {
        std::fs::write(&self.output, message.data)?;
        Ok(After::Stop)
    }
}
