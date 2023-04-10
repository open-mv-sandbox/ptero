mod utils;

use anyhow::Error;
use stewart::{Context, System};
use tracing::{event, Level};

use crate::hello_service::start_hello_service;

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();

    // Start the hello service
    let ctx = Context::root(&mut system);
    let hello = start_hello_service(ctx)?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    system.send(hello, "World");
    system.send(hello, "Actors");

    // Process messages
    system.run_until_idle()?;

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod hello_service {
    use anyhow::Error;
    use stewart::{Actor, Addr, After, Context, Id, Messages, Options, System};
    use tracing::{event, instrument, Level};

    /// The start function uses the concrete actor internally, the actor itself is never public.
    /// By instrumenting the start function, your actor's callbacks will use it automatically.
    #[instrument("hello", skip_all)]
    pub fn start_hello_service(mut ctx: Context) -> Result<Addr<String>, Error> {
        event!(Level::DEBUG, "creating service");

        let (id, mut ctx) = ctx.create()?;
        ctx.start(id, Options::default(), HelloService)?;

        Ok(Addr::new(id))
    }

    /// The actor implementation below remains entirely private to the module.
    struct HelloService;

    impl Actor for HelloService {
        type Message = String;

        fn process(
            &mut self,
            _system: &mut System,
            _id: Id,
            messages: &mut Messages<String>,
        ) -> Result<After, Error> {
            event!(Level::INFO, "processing messages");

            while let Some(message) = messages.next() {
                event!(Level::INFO, "Hello, {}!", message);
            }

            Ok(After::Continue)
        }
    }
}
