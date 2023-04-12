use anyhow::Error;
use clap::Args;
use ptero_daicon::OpenMode;
use stewart::{State, System, World};
use stewart_utils::Context;
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

    let id = ctx.register(CreateCommandSystem);

    let (id, mut ctx) = ctx.create(id)?;
    ctx.start(id, ())?;

    let file = ptero_file::open_system_file(&mut ctx, &command.target, false)?;
    ptero_daicon::open_file(&mut ctx, file, OpenMode::Create)?;

    // TODO: Receive back open success/failure

    Ok(())
}

struct CreateCommandSystem;

impl System for CreateCommandSystem {
    type Instance = ();
    type Message = ();

    fn process(&mut self, _world: &mut World, _state: &mut State<Self>) -> Result<(), Error> {
        unimplemented!()
    }
}
