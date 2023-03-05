mod commands;
mod io;

use anyhow::Error;
use clap::{Parser, Subcommand};
use stewart::{Runner, System};
use tracing::{event, Level};
use tracing_subscriber::{prelude::*, EnvFilter, FmtSubscriber};

use crate::commands::{add::AddCommand, create::CreateCommand};

fn main() {
    let filter = EnvFilter::builder()
        .parse("trace") // ,stewart=debug
        .unwrap();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish()
        .with(filter);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Run main
    let result = try_main();

    // Report any otherwise unhandled errors
    if let Err(error) = result {
        event!(Level::ERROR, "failed:\n{:?}", error);
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), Error> {
    // Parse command line args
    let args = CliArgs::parse();
    event!(Level::INFO, "running command");

    // Set up the runtime
    let mut system = System::new();
    let mut runner = Runner::new();

    // Start the command actor
    match args.command {
        Command::Create(command) => commands::create::start(&mut system, command),
        Command::Add(command) => commands::add::start(&mut system, command),
    };

    // Run the command until it's done
    runner.run_until_idle(&mut system)?;

    // TODO: Stewart doesn't currently bubble up errors for us to catch, and we need those for the
    // correct error code.
    Ok(())
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
