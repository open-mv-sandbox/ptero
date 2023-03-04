mod commands;
mod io;

use clap::{Parser, Subcommand};
use stewart::System;
use tracing::Level;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, EnvFilter, FmtSubscriber};

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

    // Parse command line args
    let args = CliArgs::parse();

    // Set up the runtime
    let mut system = System::new();

    // Start the command actor
    match args.command {
        Command::Create(command) => commands::create::start(&mut system, command),
        Command::Add(command) => commands::add::start(&mut system, command),
    };

    // Run the command until it's done
    system.run_until_idle();

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
