mod utils;

use std::sync::mpsc::channel;

use anyhow::Error;
use stewart::{Runner, System};

use crate::ping_actor::{start_ping, Ping, PingData};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();
    let mut runner = Runner::new();

    // Start the PingActor, note that it will not actually start until the system runs
    let (sender, receiver) = channel();
    start_ping(&mut system, PingData { on_start: sender });
    runner.run_until_idle(&mut system)?;

    // The PingActor should at this point have responded with an address
    let addr = receiver.try_recv().expect("PingActor didn't report start");

    // Now that we have an address, send it some data
    system.handle(addr, Ping("World"));
    system.handle(addr, Ping("Actors"));

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    system.handle(addr, Ping(data.as_str()));

    // Let the system process the messages we just sent
    runner.run_until_idle(&mut system)?;

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod ping_actor {
    use std::sync::mpsc::Sender;

    use anyhow::Error;
    use family::{Family, Member};
    use stewart::{Actor, ActorAddr, AfterProcess, AfterReduce, Start, System};
    use tracing::{event, Level};

    /// The start function uses the concrete actor internally.
    /// The actor itself is never public.
    pub fn start_ping(system: &mut System, data: PingData) {
        system.start::<PingActor>(data);
    }

    pub struct PingData {
        pub on_start: Sender<ActorAddr<PingF>>,
    }

    pub struct Ping<'a>(pub &'a str);

    // When creating a borrowed message, you need to implement the family manually

    pub enum PingF {}

    impl Family for PingF {
        type Member<'a> = Ping<'a>;
    }

    impl<'a> Member<PingF> for Ping<'a> {}

    // The actor implementation below remains entirely private to the module

    struct PingActor {
        queue: Vec<String>,
    }

    impl Start for PingActor {
        type Data = PingData;

        fn start(
            _system: &mut System,
            addr: ActorAddr<PingF>,
            data: PingData,
        ) -> Result<Self, Error> {
            event!(Level::DEBUG, "creating ping actor");
            data.on_start.send(addr).unwrap();

            Ok(Self { queue: Vec::new() })
        }
    }

    impl Actor for PingActor {
        type Family = PingF;

        fn reduce(&mut self, message: Ping) -> Result<AfterReduce, Error> {
            event!(Level::DEBUG, "adding message");

            self.queue.push(message.0.to_string());

            Ok(AfterReduce::Process)
        }

        fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
            event!(Level::DEBUG, "handling queued messages");

            for entry in self.queue.drain(..) {
                event!(Level::INFO, "Hello, {}!", entry);
            }

            // We only listen to one wave of messages then stop immediately.
            // Note though that the runtime could call this at any point after `reduce`, and messages
            // may be dropped as a result.
            Ok(AfterProcess::Stop)
        }
    }
}
