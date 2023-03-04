mod utils;

use std::sync::mpsc::channel;

use stewart::System;

use crate::ping_actor::{start_ping, Ping, PingData};

fn main() {
    utils::init_logging();

    let mut system = System::new();

    // Start the PingActor, note that it will not actually start until the system runs
    let (sender, receiver) = channel();
    start_ping(&mut system, PingData { on_start: sender });
    system.run_until_idle();

    // The PingActor should at this point have responded with an address
    let addr = receiver.try_recv().expect("PingActor didn't report start");

    // Now that we have an address, send it some data
    system.handle(addr, Ping("World"));
    system.handle(addr, Ping("Actors"));

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    system.handle(addr, Ping(data.as_str()));

    // Let the system process the messages we just sent
    system.run_until_idle();
}

/// To demonstrate encapsulation, an inner module is used here.
mod ping_actor {
    use std::sync::mpsc::Sender;

    use anyhow::Error;
    use stewart::{Actor, ActorAddrF, AfterProcess, AfterReduce, Family, StartF, System};
    use tracing::{event, Level};

    /// The start function uses the concrete actor internally.
    /// The actor itself is never public.
    pub fn start_ping(system: &mut System, data: PingData) {
        system.start_f::<PingActor>(data);
    }

    pub struct PingData {
        pub on_start: Sender<ActorAddrF<PingF>>,
    }

    pub struct Ping<'a>(pub &'a str);

    /// When creating a borrowed message family, you need to implement the family manually
    pub struct PingF;

    impl Family for PingF {
        type Member<'a> = Ping<'a>;
    }

    struct PingActor {
        queue: Vec<String>,
    }

    impl StartF for PingActor {
        type Family = PingF;
        type Data = PingData;

        fn start(
            _system: &mut System,
            addr: ActorAddrF<PingF>,
            data: PingData,
        ) -> Result<Self, Error> {
            event!(Level::DEBUG, "creating ping actor");
            data.on_start.send(addr).unwrap();

            Ok(Self { queue: Vec::new() })
        }
    }

    impl Actor for PingActor {
        type Message<'a> = Ping<'a>;

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
