mod utils;

use anyhow::Error;
use stewart::System;
use tracing::{event, Level};

use crate::hello_service::start_hello_service;

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();

    // Start the hello service
    let hello = start_hello_service(&mut system.root())?;

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
    use stewart::{Actor, Addr, After, Context, Options};
    use tracing::{event, instrument, Level};

    /// The start function uses the concrete actor internally, the actor itself is never public.
    /// By instrumenting the start function, your actor's callbacks will use it automatically.
    #[instrument("hello", skip_all)]
    pub fn start_hello_service(ctx: &mut Context) -> Result<Addr<String>, Error> {
        event!(Level::DEBUG, "creating service");

        let mut ctx = ctx.create()?;
        ctx.start(Options::default(), HelloService)?;

        Ok(ctx.addr()?)
    }

    /// The actor implementation below remains entirely private to the module.
    struct HelloService;

    impl Actor for HelloService {
        type Message = String;

        fn handle(&mut self, _ctx: &mut Context, message: String) -> Result<After, Error> {
            event!(Level::INFO, "Hello, {}!", message);

            Ok(After::Continue)
        }
    }
}
