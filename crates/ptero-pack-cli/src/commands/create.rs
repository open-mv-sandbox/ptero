use anyhow::Error;
use clap::Args;
use stewart::{utils::ActorT, AfterProcess, AfterReduce, System};
use tracing::{event, instrument, Level};

/// Create a new dacti package.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// The path to create the package at.
    #[arg(short, long, value_name = "PATH")]
    package: String,
}

#[instrument("create-command", skip_all)]
pub fn start(system: &mut System, data: CreateCommand) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let addr = system.create();
    system.start(addr, CreateCommandActor)?;

    ptero_pack::create_package(&data.package)?;

    Ok(())
}

struct CreateCommandActor;

impl ActorT for CreateCommandActor {
    type Message = ();

    fn reduce(&mut self, _message: ()) -> Result<AfterReduce, Error> {
        unimplemented!()
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        // TODO: Currently makes no sense for this to be an actor, but it will use other actors
        unimplemented!()
    }
}
