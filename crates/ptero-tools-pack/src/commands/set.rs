use anyhow::Error;
use clap::Args;
use ptero_daicon::{OpenMode, SourceAction, SourceMessage};
use stewart::{Actor, Addr, After, Context, Id, Options, System};
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
pub fn start(mut ctx: Context, command: SetCommand) -> Result<(), Error> {
    event!(Level::INFO, "setting file in package");

    let (id, mut ctx) = ctx.create()?;

    let data = std::fs::read(&command.input)?;

    // Open up the package for writing in ptero-daicon
    let file = ptero_file::open_system_file(&mut ctx, &command.target, false)?;
    let source = ptero_daicon::open_file(&mut ctx, file, OpenMode::ReadWrite)?;

    // Add the data to the source
    let message = SourceMessage {
        id: Uuid::new_v4(),
        action: SourceAction::Set {
            id: command.id,
            data,
            on_result: Addr::new(id),
        },
    };
    ctx.send(source, message);

    ctx.start(id, Options::default(), AddCommandActor)?;

    Ok(())
}

struct AddCommandActor;

impl Actor for AddCommandActor {
    type Message = ();

    fn handle(&mut self, _system: &mut System, _id: Id, _message: ()) -> Result<After, Error> {
        Ok(After::Stop)
    }
}
