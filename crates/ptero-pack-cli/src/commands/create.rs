use anyhow::Error;
use clap::Args;
use stewart::System;
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

    let info = system.create_actor(system.root_id())?;
    system.start_actor(info, CreateCommandActor)?;

    ptero_pack::create_package(&data.package)?;

    Ok(())
}

struct CreateCommandActor;
