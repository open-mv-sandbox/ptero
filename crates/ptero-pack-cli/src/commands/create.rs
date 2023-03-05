use anyhow::Error;
use clap::Args;
use stewart::{
    utils::{ActorAddrT, ActorT, SystemExt, Void},
    AfterProcess, AfterReduce, System,
};
use tracing::{event, Level};

/// Create a new dacti package.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// The path to create the package at.
    #[arg(short, long, value_name = "PATH")]
    package: String,
}

pub fn start(system: &mut System, data: CreateCommand) {
    system.start_with("ppcli-create", data, CreateCommandActor::start);
}

struct CreateCommandActor;

impl CreateCommandActor {
    fn start(
        _system: &mut System,
        _addr: ActorAddrT<Void>,
        data: CreateCommand,
    ) -> Result<Self, Error> {
        event!(Level::INFO, "creating package");

        ptero_pack::create_package(&data.package)?;

        Ok(CreateCommandActor)
    }
}

impl ActorT for CreateCommandActor {
    type Message = Void;

    fn reduce(&mut self, _message: Void) -> Result<AfterReduce, Error> {
        unimplemented!()
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        // TODO: Currently makes no sense for this to be an actor, but it will use other actors
        unimplemented!()
    }
}
