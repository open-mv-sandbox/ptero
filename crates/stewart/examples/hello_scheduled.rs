mod utils;

use anyhow::Error;
use stewart::{schedule::Schedule, System};
use tracing::{event, Level};

use crate::hello_serivce::{start_hello, HelloMsg};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();
    let schedule = Schedule::new();

    // Start the hello service
    let sender = start_hello(&mut system, None, schedule.clone())?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    sender.send(&mut system, HelloMsg("World"));
    sender.send(&mut system, HelloMsg("Actors"));

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    sender.send(&mut system, HelloMsg(data.as_str()));

    // Process actors until idle
    event!(Level::DEBUG, "processing actors");
    schedule.run_until_idle(&mut system)?;

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod hello_serivce {
    use anyhow::Error;
    use family::Member;
    use stewart::{
        handler::{Handler, Sender},
        schedule::{Process, Schedule},
        After, Id, Info, System,
    };
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
        parent: Option<Id>,
        schedule: Schedule,
    ) -> Result<Sender<HelloMsgF>, Error> {
        event!(Level::DEBUG, "creating service");

        let info = system.create_actor(parent)?;
        let actor = HelloActor {
            info,
            queue: Vec::new(),
            schedule,
        };
        system.start_actor(info, actor)?;

        Ok(Sender::new(info))
    }

    /// The actor implementation below remains entirely private to the module.
    struct HelloActor {
        info: Info<Self>,
        queue: Vec<String>,
        schedule: Schedule,
    }

    impl Handler for HelloActor {
        type Family = HelloMsgF;

        fn handle(&mut self, _system: &mut System, message: HelloMsg) -> Result<After, Error> {
            event!(Level::DEBUG, "queuing message");

            // Record the message on the actor's processing queue
            self.queue.push(message.0.to_string());

            // Add this actor to the schedule for processing
            self.schedule.push(self.info)?;

            Ok(After::Nothing)
        }
    }

    impl Process for HelloActor {
        fn process(&mut self, _system: &mut System) -> Result<After, Error> {
            event!(Level::DEBUG, "processing scheduled messages");

            for entry in self.queue.drain(..) {
                event!(Level::INFO, "Hello, {}!", entry);
            }

            Ok(After::Nothing)
        }
    }
}
