use anyhow::Error;
use clap::Args;
use ptero_daicon::{FileSourceApi, OpenMode};
use ptero_file::SystemFileApi;
use stewart::{State, System, SystemOptions, World};
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

    let file_api = SystemFileApi::new(&mut ctx);
    let source_api = FileSourceApi::new(&mut ctx);

    let system = ctx.register(SystemOptions::default(), CreateCommandSystem);

    let (id, mut ctx) = ctx.create()?;
    ctx.start(id, system, ())?;

    let file = file_api.open(&mut ctx, &command.target, false)?;
    source_api.open(&mut ctx, file, OpenMode::Create)?;

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
