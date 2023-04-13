use anyhow::{Context as _, Error};
use clap::Args;
use ptero_daicon::{OpenMode, SourceAction, SourceMessage};
use ptero_file::{ReadResult, SystemFile};
use stewart::{Addr, State, System, SystemOptions, World};
use stewart_utils::Context;
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

    let id = ctx.register(SystemOptions::default(), GetCommandSystem);
    let (id, mut ctx) = ctx.create(id)?;

    // Open up the package for writing in ptero-daicon
    let file = SystemFile::new(&mut ctx).open(&mut ctx, &command.target, false)?;
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

    ctx.start(id, command)?;

    Ok(())
}

struct GetCommandSystem;

impl System for GetCommandSystem {
    type Instance = GetCommand;
    type Message = ReadResult;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((actor, message)) = state.next() {
            let instance = state.get_mut(actor).context("failed to get instance")?;
            std::fs::write(&instance.output, message.data)?;
            world.stop(actor)?;
        }

        Ok(())
    }
}
