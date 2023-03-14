use anyhow::Error;
use clap::Args;
use ptero_pack::{start_package_manager, PackageManagerCommand};
use stewart::{Actor, After, Options, System};
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

    let info = system.create(system.root_id())?;
    system.start(info, Options::default(), CreateCommandActor)?;

    let package = start_package_manager(system, info.id())?;
    system.send(package, PackageManagerCommand::Create(data.package));

    Ok(())
}

struct CreateCommandActor;

impl Actor for CreateCommandActor {
    type Message = ();

    fn handle(&mut self, _system: &mut System, _message: ()) -> Result<After, Error> {
        unimplemented!()
    }
}
