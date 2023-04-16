use anyhow::Error;
use clap::Args;
use ptero_daicon::{FileSourceApi, OpenMode};
use ptero_file::{FileMessage, SystemFileServiceMessage};
use stewart::{Addr, State, System, SystemOptions, World};
use stewart_utils::map_once;
use tracing::{event, instrument, Level};

/// Create a new daicon file.
#[derive(Args, Debug)]
pub struct CreateCommand {
    /// Path of the target file.
    #[arg(short, long, value_name = "PATH")]
    target: String,
}

#[instrument("create-command", skip_all)]
pub fn start(
    world: &mut World,
    system_file: Addr<SystemFileServiceMessage>,
    command: CreateCommand,
) -> Result<(), Error> {
    event!(Level::INFO, "creating package");

    let actor = world.create(None)?;
    let addr = Addr::new(actor);

    // Open the target file
    let message = SystemFileServiceMessage::Open {
        parent: Some(actor),
        path: command.target.clone(),
        truncate: true,
        on_result: map_once(world, Some(actor), addr, Message::FileOpened)?,
    };
    world.send(system_file, message);

    // Start the command actor
    let system = world.register(SystemOptions::default(), CreateCommandSystem);
    world.start(actor, system, command)?;

    Ok(())
}

struct CreateCommandSystem;

impl System for CreateCommandSystem {
    type Instance = CreateCommand;
    type Message = Message;

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((actor, message)) = state.next() {
            let Message::FileOpened(file) = message;

            let source_api = FileSourceApi::new(world);
            source_api.open(world, Some(actor), file, OpenMode::Create)?;
        }

        Ok(())
    }
}

enum Message {
    FileOpened(Addr<FileMessage>),
}
