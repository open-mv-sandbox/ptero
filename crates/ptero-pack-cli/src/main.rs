mod commands;
mod io;

use clap::{Parser, Subcommand};
use stewart::Factory;
use stewart_native::{ThreadExecutor, World};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use crate::commands::{add::AddCommand, create::CreateCommand};

fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Parse command line args
    let args = CliArgs::parse();

    // Set up the runtime
    let world = World::new();

    // Start the command actor
    let command: Box<dyn Factory> = match args.command {
        Command::Create(command) => Box::new(command),
        Command::Add(command) => Box::new(command),
    };

    // Run the command until it's done
    world.start(command);
    let executor = ThreadExecutor::new(world);
    executor.run_until_idle();

    // TODO: Stewart doesn't currently bubble up errors for us to catch, and we need those for the
    // correct error code.
    /*if let Err(error) = result {
        event!(Level::ERROR, "failed:\n{:?}", error);
        std::process::exit(1);
    }*/
}

/// Pterodactil CLI toolkit for working with dacti packages.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Create(CreateCommand),
    Add(AddCommand),
}