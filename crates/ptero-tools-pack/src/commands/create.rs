use anyhow::Error;
use clap::Args;
use stewart::{Actor, After, Context, Options};
use tracing::{event, instrument, Level};

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("create-command", skip_all)]
pub fn start(ctx: &mut Context, command: CreateCommand) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let mut ctx = ctx.create()?;
    ctx.start(Options::default(), CreateCommandActor)?;

    let file = ptero_file::start_system_file(&mut ctx, &command.target, false)?;
    ptero_daicon::open_file(&mut ctx, file, true)?;

    // TODO: Receive back open success/failure

    Ok(())
}

struct CreateCommandActor;

impl Actor for CreateCommandActor {
    type Message = ();

    fn handle(&mut self, _ctx: &mut Context, _message: ()) -> Result<After, Error> {
        unimplemented!()
    }
}
