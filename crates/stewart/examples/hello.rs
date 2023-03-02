mod utils;

use std::sync::mpsc::{channel, Sender};

use better_any::{Tid, TidAble};
use stewart::{Actor, AfterReduce, Factory, System, SystemAddr};
use tracing::{event, Level};

fn main() {
    utils::init_logging();

    let mut system = System::new();

    // Start the PingActor, note that it will not actually start until the system runs
    let (sender, receiver) = channel();
    system.start(PingData { on_start: sender });
    system.run_until_idle();

    // The PingActor should at this point have responded with an address
    let addr = receiver.try_recv().expect("PingActor didn't report start");

    // Now that we have an address, send it some data
    system.handle(addr, Ping("World"));
    system.handle(addr, Ping("Actors"));

    // Let the system process the messages we just sent
    system.run_until_idle();
}

#[derive(Factory)]
#[factory(PingActor::start)]
struct PingData {
    on_start: Sender<SystemAddr<Ping<'static>>>,
}

struct PingActor {
    queue: Vec<String>,
}

impl PingActor {
    pub fn start(addr: SystemAddr<Ping<'static>>, data: PingData) -> Self {
        event!(Level::DEBUG, "creating ping actor");
        data.on_start.send(addr).unwrap();

        Self { queue: Vec::new() }
    }
}

impl Actor for PingActor {
    type Message<'a> = Ping<'a>;

    fn reduce(&mut self, message: Ping) -> AfterReduce {
        event!(Level::DEBUG, "adding message");

        self.queue.push(message.0.to_string());

        AfterReduce::Process
    }

    fn process(&mut self) {
        event!(Level::DEBUG, "handling queued messages");

        for entry in self.queue.drain(..) {
            event!(Level::INFO, "Hello, {}!", entry);
        }
    }
}

#[derive(Tid)]
struct Ping<'a>(&'a str);
