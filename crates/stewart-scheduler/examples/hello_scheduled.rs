mod utils;

use anyhow::Error;
use stewart::{Id, System};
use stewart_scheduler::{run_until_idle, start_scheduler};
use tracing::{event, Level};

use crate::hello_serivce::{start_hello, HelloMsg};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();

    // Start the hello service
    let process = start_scheduler(&mut system, Id::root())?;
    let addr = start_hello(&mut system, Id::root(), process)?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    system.handle(addr, HelloMsg("World"));
    system.handle(addr, HelloMsg("Actors"));

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    system.handle(addr, HelloMsg(data.as_str()));

    // Process actors until idle
    event!(Level::DEBUG, "processing actors");
    run_until_idle(&mut system, process)?;

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod hello_serivce {
    use anyhow::Error;
    use family::Member;
    use stewart::{Actor, Addr, AddrT, After, Id, Info, System};
    use stewart_scheduler::{Process, ProcessItem};
    use tracing::{event, instrument, Level};

    /// When creating a borrowed message, you need to implement the `Member` and `Family` traits.
    /// For common cases, you can just use the derive macro, however you can do this yourself too.
    #[derive(Member)]
    pub struct HelloMsg<'a>(pub &'a str);

    /// The start function uses the concrete actor internally, the actor itself is never public.
    /// By instrumenting the start function, your actor's callbacks will use it automatically.
    #[instrument("hello", skip_all)]
    pub fn start_hello(
        system: &mut System,
        parent: Id,
        process: AddrT<ProcessItem>,
    ) -> Result<Addr<HelloMsgF>, Error> {
        event!(Level::DEBUG, "creating service");

        let info = system.create_actor(parent)?;
        let actor = HelloActor {
            info,
            queue: Vec::new(),
            process,
        };
        system.start_actor(info, actor)?;

        Ok(info.addr())
    }

    /// The actor implementation below remains entirely private to the module.
    struct HelloActor {
        info: Info<Self>,
        queue: Vec<String>,
        process: AddrT<ProcessItem>,
    }

    impl Actor for HelloActor {
        type Family = HelloMsgF;

        fn handle(&mut self, system: &mut System, message: HelloMsg) -> Result<After, Error> {
            event!(Level::DEBUG, "queuing message");

            self.queue.push(message.0.to_string());

            let message = ProcessItem::new::<Self>(self.info.id());
            system.handle(self.process, message);

            Ok(After::Nothing)
        }
    }

    impl Process for HelloActor {
        fn process(&mut self, _system: &mut System) -> Result<After, Error> {
            for entry in self.queue.drain(..) {
                event!(Level::INFO, "Hello, {}!", entry);
            }

            Ok(After::Nothing)
        }
    }
}
