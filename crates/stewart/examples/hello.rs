mod utils;

use anyhow::Error;
use stewart::World;
use tracing::{event, Level};

use crate::hello_service::HelloServiceApi;

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut world = World::new();

    // Start the hello service
    let hello = HelloServiceApi::new(&mut world);
    let service = hello.start(&mut world, "Example".to_string())?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    world.send(service, "World");
    world.send(service, "Actors");

    // Process messages
    world.run_until_idle()?;

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod hello_service {
    use anyhow::{Context, Error};
    use stewart::{Addr, State, System, SystemId, SystemOptions, World};
    use tracing::{event, instrument, span, Level};

    /// The entrypoint of the Hello Service's API.
    #[derive(Clone)]
    pub struct HelloServiceApi {
        system: SystemId,
    }

    impl HelloServiceApi {
        pub fn new(world: &mut World) -> Self {
            Self {
                system: world.register(SystemOptions::default(), HelloServiceSystem),
            }
        }

        #[instrument("hello-service", skip_all, fields(name = name))]
        pub fn start(&self, world: &mut World, name: String) -> Result<Addr<String>, Error> {
            event!(Level::DEBUG, "creating service");

            // stewart_utils provides a `Context` helper that automatically tracks current parent
            // for creation, but you are not required to use this.
            let id = world.create(self.system, None)?;
            let instance = HelloService { name };
            world.start(id, instance)?;

            Ok(Addr::new(id))
        }
    }

    // The actor implementation below remains entirely private to the module.

    struct HelloServiceSystem;

    impl System for HelloServiceSystem {
        type Instance = HelloService;
        type Message = String;

        fn process(&mut self, _world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
            event!(Level::INFO, "processing messages");

            while let Some((id, message)) = state.next() {
                let instance = state.get_mut(id).context("failed to get instance")?;

                let span = span!(Level::INFO, "hello-service", name = instance.name);
                let _enter = span.enter();

                event!(Level::INFO, "Hello, {} from {}!", message, instance.name);
            }

            Ok(())
        }
    }

    struct HelloService {
        name: String,
    }
}
