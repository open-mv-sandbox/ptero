use anyhow::Error;
use stewart::{Actor, Addr, After, Id, Options, System};
use tracing::{event, instrument, Level};

pub enum PackageManagerCommand {
    Create(String),
}

#[instrument("package-manager", skip_all)]
pub fn start_package_manager(
    system: &mut System,
    parent: Id,
) -> Result<Addr<PackageManagerCommand>, Error> {
    let info = system.create(parent)?;
    system.start(info, PackageManagerActor {}, Options::default())?;

    Ok(info.addr())
}

struct PackageManagerActor {}

impl Actor for PackageManagerActor {
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
