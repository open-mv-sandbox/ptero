use anyhow::Error;
use clap::Args;
use ptero_daicon::OpenMode;
use stewart::{Actor, After, Context, Id, Messages, Options, System};
use tracing::{event, instrument, Level};

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("create-command", skip_all)]
pub fn start(mut ctx: Context, command: CreateCommand) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let (id, mut ctx) = ctx.create()?;
    ctx.start(id, Options::default(), CreateCommandActor)?;

    let file = ptero_file::open_system_file(&mut ctx, &command.target, false)?;
    ptero_daicon::open_file(&mut ctx, file, OpenMode::Create)?;

    // TODO: Receive back open success/failure

    Ok(())
}

struct CreateCommandActor;

impl Actor for CreateCommandActor {
    type Message = ();

    fn process(
        &mut self,
        _system: &mut System,
        _id: Id,
        _messages: &mut Messages<()>,
    ) -> Result<After, Error> {
        unimplemented!()
    }
}
