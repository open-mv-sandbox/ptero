use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::Error;
use stewart::{Actor, Addr, After, Options, System};
use tracing_test::traced_test;

#[test]
#[traced_test]
fn send_message_to_actor() -> Result<(), Error> {
    let mut system = System::new();
    let (addr, count) = start_actor(&mut system)?;

    // Send a message
    system.send(addr, ());
    system.run_until_idle()?;

    // Actor will now have removed itself, send again to make sure it doesn't crash
    system.send(addr, ());
    system.run_until_idle()?;

    // Make sure it wasn't handled anyways
    assert_eq!(count.load(Ordering::SeqCst), 1);

    Ok(())
}

fn start_actor(system: &mut System) -> Result<(Addr<()>, Rc<AtomicUsize>), Error> {
    let (id, addr) = system.create_root()?;
    let actor = TestActor::default();
    let count = actor.count.clone();
    system.start(id, Options::default(), actor)?;
    Ok((addr, count))
}

#[derive(Default)]
struct TestActor {
    count: Rc<AtomicUsize>,
}

impl Actor for TestActor {
    type Message = ();

    fn handle(&mut self, _system: &mut System, _message: ()) -> Result<After, Error> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(After::Stop)
    }
}
