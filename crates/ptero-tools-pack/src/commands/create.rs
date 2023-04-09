use std::{fs::OpenOptions, io::Write};

use anyhow::{Context as _, Error};
use bytemuck::{bytes_of, Zeroable};
use clap::Args;
use daicon::{Entry, Header};
use stewart::{Actor, After, Context, Options};
use tracing::{event, instrument, Level};

/// TODO: Restructure to use actors and add to ptero_daicon
fn create_package(path: &str) -> Result<(), Error> {
    // Open the target file, overwriting anything already there
    let mut package = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .context("failed to open target package for writing")?;

    // Write the component table
    let mut header = Header::default();
    header.set_capacity(256);
    package.write_all(bytes_of(&header))?;

    // Write an empty entries table
    for _ in 0..256 {
        let entry = Entry::zeroed();
        package.write_all(bytes_of(&entry))?;
    }

    Ok(())
}

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

    create_package(&command.target)?;

    Ok(())
}

struct CreateCommandActor;

impl Actor for CreateCommandActor {
    type Message = ();

    fn handle(&mut self, _ctx: &mut Context, _message: ()) -> Result<After, Error> {
        unimplemented!()
    }
}
