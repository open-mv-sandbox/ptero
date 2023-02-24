use anyhow::Error;
use clap::Args;
use stewart::Actor;
use stewart_local::{Address, Context, Factory};
use tracing::{event, Level};

/// Create a new dacti package.
#[derive(Factory, Args, Debug)]
#[factory(CreateCommandActor::start)]
pub struct CreateCommand {
    /// The path to create the package at.
    #[arg(short, long, value_name = "PATH")]
    package: String,
}

struct CreateCommandActor;

impl CreateCommandActor {
    pub fn start(_ctx: Context, _address: Address<()>, data: CreateCommand) -> Result<Self, Error> {
        event!(Level::INFO, "creating package");

        ptero_pack::create_package(&data.package)?;

        Ok(CreateCommandActor)
    }
}

impl Actor for CreateCommandActor {
    type Message = ();

    fn handle(&mut self, _message: Self::Message) -> Result<stewart::Next, anyhow::Error> {
        // TODO: Currently makes no sense for this to be an actor, but it will use other actors
        unimplemented!()
    }
}
