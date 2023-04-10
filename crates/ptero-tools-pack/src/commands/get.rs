use anyhow::Error;
use clap::Args;
use ptero_daicon::{OpenMode, SourceAction, SourceMessage};
use ptero_file::ReadResult;
use stewart::{Actor, ActorData, Addr, After, Context, Id, Options, System};
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

#[instrument("get-command", skip_all)]
pub fn start(mut ctx: Context, command: GetCommand) -> Result<(), Error> {
    event!(Level::INFO, "getting file from package");

    let (id, mut ctx) = ctx.create()?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::open_system_file(&mut ctx, &command.target, false)?;
    let source = ptero_daicon::open_file(&mut ctx, file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Get {
            id: command.id,
            on_result: Addr::new(id),
        },
    };
    ctx.send(source, message);

    let actor = GetCommandActor {
        output: command.output,
    };
    ctx.start(id, Options::default(), actor)?;

    Ok(())
}

struct GetCommandActor {
    output: String,
}

impl Actor for GetCommandActor {
    type Message = ReadResult;

    fn process(
        &mut self,
        _system: &mut System,
        _id: Id,
        data: &mut ActorData<ReadResult>,
    ) -> Result<After, Error> {
        while let Some(message) = data.next() {
            std::fs::write(&self.output, message.data)?;
        }

        Ok(After::Stop)
    }
}
