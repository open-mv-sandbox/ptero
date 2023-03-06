mod utils;

use anyhow::Error;
use stewart::System;

use crate::ping_actor::{start_ping, Ping};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();

    // Start the PingActor, note that it will not actually start until the system runs
    let addr = start_ping(&mut system);
    system.run_until_idle()?;

    // Now that we have an address, send it some data
    system.handle(addr, Ping("World"))?;
    system.handle(addr, Ping("Actors"))?;

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    system.handle(addr, Ping(data.as_str()))?;

    // Let the system process the messages we just sent
    system.run_until_idle()?;

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod ping_actor {
    use anyhow::Error;
    use family::{Family, Member};
    use stewart::{Actor, Addr, AfterProcess, AfterReduce, System};
    use tracing::{event, Level};

    /// The start function uses the concrete actor internally.
    /// The actor itself is never public.
    pub fn start_ping(system: &mut System) -> Addr<PingF> {
        system.start("ping", PingActor::start)
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

    impl PingActor {
        fn start(_system: &mut System, _addr: Addr<PingF>) -> Result<Self, Error> {
            event!(Level::DEBUG, "creating ping actor");

            Ok(Self { queue: Vec::new() })
        }
    }

    impl Actor for PingActor {
        type Family = PingF;

        fn reduce(&mut self, message: Ping) -> Result<AfterReduce, Error> {
            event!(Level::DEBUG, "adding message");

            // Because "Ping" is a borrowed value, you have to decide how to most efficiently
            // queue it yourself in your actor.
            self.queue.push(message.0.to_string());

            Ok(AfterReduce::Process)
        }

        fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
            event!(Level::DEBUG, "handling queued messages");

            // Process the messages previously queued
            for entry in self.queue.drain(..) {
                event!(Level::INFO, "Hello, {}!", entry);
            }

            // We only listen to one wave of messages then stop immediately.
            // Note though that the runtime could call this at any point after `reduce`, and
            // messages may be dropped as a result.
            Ok(AfterProcess::Stop)
        }
    }
}
