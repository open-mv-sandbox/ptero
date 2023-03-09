mod utils;

use anyhow::Error;
use stewart::{Id, System};
use tracing::{event, Level};

use crate::hello_serivce::{start_hello, HelloMsg};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();

    // Start the hello service
    let addr = start_hello(&mut system, Id::root())?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    system.handle(addr, HelloMsg("World"));
    system.handle(addr, HelloMsg("Actors"));

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    system.handle(addr, HelloMsg(data.as_str()));

    // Let the system process the messages we just sent
    system.run_until_idle()?;
    event!(Level::INFO, "finished executing actors");

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod hello_serivce {
    use anyhow::Error;
    use family::Member;
    use stewart::{Actor, Addr, After, Id, System};
    use tracing::{event, instrument, Level};

    /// The start function uses the concrete actor internally, the actor itself is never public.
    /// By instrumenting the start function, your actor's callbacks will use it automatically.
    #[instrument("hello", skip_all)]
    pub fn start_hello(system: &mut System, parent: Id) -> Result<Addr<HelloMsgF>, Error> {
        event!(Level::DEBUG, "creating service");

        let info = system.create_actor(parent)?;
        let actor = HelloActor { queue: Vec::new() };
        system.start_actor(info, actor)?;

        Ok(info.addr())
    }

    /// When creating a borrowed message, you need to implement the `Member` and `Family` traits.
    /// For common cases, you can just use the derive macro, however you can do this yourself too.
    #[derive(Member)]
    pub struct HelloMsg<'a>(pub &'a str);

    /// The actor implementation below remains entirely private to the module.
    struct HelloActor {
        queue: Vec<String>,
    }

    impl Actor for HelloActor {
        type Family = HelloMsgF;

        fn reduce(&mut self, _system: &mut System, message: HelloMsg) -> Result<After, Error> {
            event!(Level::DEBUG, "adding message");

            // Because "HelloMsg" is a borrowed value, you have to decide how to most
            // efficiently queue it yourself in your actor.
            self.queue.push(message.0.to_string());

            Ok(After::Process)
        }

        fn process(&mut self, _system: &mut System) -> Result<After, Error> {
            event!(Level::DEBUG, "handling queued messages");

            // Process the messages previously queued
            for entry in self.queue.drain(..) {
                event!(Level::INFO, "Hello, {}!", entry);
            }

            // We only listen to one wave of messages then stop immediately.
            // Note though that the runtime could call this at any point after `reduce`, and
            // messages may be dropped as a result.
            Ok(After::Stop)
        }
    }
}
