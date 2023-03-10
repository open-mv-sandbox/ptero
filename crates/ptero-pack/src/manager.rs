use anyhow::Error;
use stewart::{
    handler::{HandlerT, SenderT},
    After, Id, System,
};
use tracing::{event, instrument, Level};

pub enum PackageManagerCommand {
    Create(String),
}

#[instrument("package-manager", skip_all)]
pub fn start_package_manager(
    system: &mut System,
    parent: Id,
) -> Result<SenderT<PackageManagerCommand>, Error> {
    let info = system.create_actor(parent)?;
    system.start_actor(info, PackageManagerActor {})?;

    Ok(SenderT::actor(info))
}

struct PackageManagerActor {}

impl HandlerT for PackageManagerActor {
    type Message = PackageManagerCommand;

    fn handle(&mut self, _system: &mut System, message: Self::Message) -> Result<After, Error> {
        match message {
            PackageManagerCommand::Create(path) => {
                event!(Level::INFO, "creating new package");
                crate::create_package(&path)?;
            }
        }

        Ok(After::Nothing)
    }
}
