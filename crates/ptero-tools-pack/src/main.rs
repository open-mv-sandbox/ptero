mod commands;

use anyhow::Error;
use clap::{Parser, Subcommand};
use commands::get::GetCommand;
use stewart::System;
use tracing::{event, Level};
use tracing_subscriber::{prelude::*, EnvFilter, FmtSubscriber};

use crate::commands::{create::CreateCommand, set::SetCommand};

fn main() {
    let args = CliArgs::parse();

    let filter = EnvFilter::builder()
        .parse("trace,stewart=warn,ptero_file=warn")
        .unwrap();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .without_time()
        .with_target(false)
        .finish()
        .with(filter);
    tracing::subscriber::set_global_default(subscriber).unwrap();

    // Run main
    let result = try_main(args);

    // Report any otherwise unhandled errors
    if let Err(error) = result {
        event!(Level::ERROR, "failed:\n{:?}", error);
        std::process::exit(1);
    }
}

fn try_main(args: CliArgs) -> Result<(), Error> {
    // Set up the runtime
    let mut system = System::new();
    let mut ctx = system.root();

    // Start the command actor
    match args.command {
        Command::Create(command) => commands::create::start(&mut ctx, command)?,
        Command::Set(command) => commands::set::start(&mut ctx, command)?,
        Command::Get(command) => commands::get::start(&mut ctx, command)?,
    };

    // Run the command until it's done
    system.run_until_idle()?;

    // TODO: Receive command errors
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
    Set(SetCommand),
    Get(GetCommand),
}
