mod utils;

use anyhow::Error;
use stewart::{Id, System};
use tracing::{event, Level};

use crate::hello_serivce::{start_hello, HelloMsg};

fn main() -> Result<(), Error> {
    utils::init_logging();

    let mut system = System::new();

    // Start the hello service
    let sender = start_hello(&mut system, Id::root())?;

    // Now that we have an address, send it some data
    event!(Level::INFO, "sending messages");
    sender.send(&mut system, HelloMsg("World"));
    sender.send(&mut system, HelloMsg("Actors"));

    // You can also use temporary borrows!
    let data = String::from("Borrowed");
    sender.send(&mut system, HelloMsg(data.as_str()));

    Ok(())
}

/// To demonstrate encapsulation, an inner module is used here.
mod hello_serivce {
    use anyhow::Error;
    use family::Member;
    use stewart::{Actor, After, Id, Sender, System};
    use tracing::{event, instrument, Level};

    /// When creating a borrowed message, you need to implement the `Member` and `Family` traits.
    /// For common cases, you can just use the derive macro, however you can do this yourself too.
    #[derive(Member)]
    pub struct HelloMsg<'a>(pub &'a str);

    /// The start function uses the concrete actor internally, the actor itself is never public.
    /// By instrumenting the start function, your actor's callbacks will use it automatically.
    #[instrument("hello", skip_all)]
    pub fn start_hello(system: &mut System, parent: Id) -> Result<Sender<HelloMsgF>, Error> {
        event!(Level::DEBUG, "creating service");

        let info = system.create_actor(parent)?;
        system.start_actor(info, HelloActor)?;

        Ok(info.sender())
    }

    /// The actor implementation below remains entirely private to the module.
    struct HelloActor;

    impl Actor for HelloActor {
        type Family = HelloMsgF;

        fn handle(&mut self, _system: &mut System, message: HelloMsg) -> Result<After, Error> {
            event!(Level::INFO, "Hello, {}!", message.0);

            Ok(After::Nothing)
        }
    }
}
