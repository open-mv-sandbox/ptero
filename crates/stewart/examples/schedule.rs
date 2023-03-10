mod utils;

use anyhow::Error;
use stewart::{schedule::Schedule, System};
use tracing::{event, Level};

use crate::hello_serivce::{start_schedule_hello, HelloMsg};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();
    let schedule = Schedule::new();

    // Start the hello service
    let root = system.root_id();
    let sender = start_schedule_hello(&mut system, root, schedule.clone())?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    sender.send(&mut system, HelloMsg("World"));
    sender.send(&mut system, HelloMsg("Actors"));
    sender.send(&mut system, HelloMsg("Schedule"));
    sender.send(&mut system, HelloMsg("All At Once"));

    // Process actors until idle
    event!(Level::DEBUG, "processing actors");
    schedule.run_until_idle(&mut system)?;

    Ok(())
}

mod hello_serivce {
    use anyhow::Error;
    use family::Member;
    use stewart::{
        handler::{Handler, Sender},
        schedule::{Process, Schedule},
        After, Id, Info, System,
    };
    use tracing::{event, instrument, Level};

    #[derive(Member)]
    pub struct HelloMsg<'a>(pub &'a str);

    #[instrument("schedule-hello", skip_all)]
    pub fn start_schedule_hello(
        system: &mut System,
        parent: Id,
        schedule: Schedule,
    ) -> Result<Sender<HelloMsgF>, Error> {
        event!(Level::DEBUG, "creating service");

        let info = system.create_actor(parent)?;
        let actor = ScheduleHelloActor {
            info,
            queue: Vec::new(),
            schedule,
        };
        system.start_actor(info, actor)?;

        Ok(Sender::actor(info))
    }

    struct ScheduleHelloActor {
        info: Info<Self>,
        queue: Vec<String>,
        schedule: Schedule,
    }

    impl Handler for ScheduleHelloActor {
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

    // Schedule processing looks for the `Process` trait on your actor
    impl Process for ScheduleHelloActor {
        fn process(&mut self, _system: &mut System) -> Result<After, Error> {
            event!(Level::DEBUG, "processing scheduled messages");

            let entries = self.queue.join(", ");
            event!(Level::INFO, "Hello, {}!", entries);
            self.queue.clear();

            Ok(After::Nothing)
        }
    }
}
