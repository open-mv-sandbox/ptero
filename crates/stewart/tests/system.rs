use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::Error;
use rstest::{fixture, rstest};
use stewart::{Actor, Addr, After, Context, Id, Options, System};
use tracing_test::traced_test;

#[rstest]
#[traced_test]
fn system_sends_message_to_actor(mut world: TestWorld) -> Result<(), Error> {
    // Send a message
    world.system.send(world.root.addr, ());
    world.system.run_until_idle()?;

    // Actor will now have removed itself, send again to make sure it doesn't crash
    world.system.send(world.root.addr, ());
    world.system.run_until_idle()?;

    // Make sure the first message was handled, but not the second one
    assert_eq!(world.root.count.load(Ordering::SeqCst), 1);

    Ok(())
}

#[rstest]
#[traced_test]
fn system_stops_actors(mut world: TestWorld) -> Result<(), Error> {
    // Send a message to the part
    world.system.send(world.root.addr, ());
    world.system.run_until_idle()?;

    // Actor will now have removed itself, try sending a message to the child
    world.system.send(world.child.addr, ());
    world.system.run_until_idle()?;

    // Make sure it wasn't handled
    assert_eq!(world.child.count.load(Ordering::SeqCst), 0);

    Ok(())
}

#[fixture]
fn world() -> TestWorld {
    let mut system = System::new();
    let mut ctx = Context::root(&mut system);

    let (root, mut ctx) = create_actor(&mut ctx);
    let (child, _) = create_actor(&mut ctx);

    TestWorld {
        system,
        root,
        child,
    }
}

fn create_actor<'a>(ctx: &'a mut Context) -> (ActorInfo, Context<'a>) {
    let (id, mut ctx) = ctx.create().unwrap();
    let actor = TestActor::default();
    let count = actor.count.clone();
    ctx.start(id, Options::default(), actor).unwrap();

    let info = ActorInfo {
        addr: Addr::new(id),
        count,
    };

    (info, ctx)
}

struct TestWorld {
    system: System,
    root: ActorInfo,
    child: ActorInfo,
}

struct ActorInfo {
    addr: Addr<()>,
    count: Rc<AtomicUsize>,
}

#[derive(Default)]
struct TestActor {
    count: Rc<AtomicUsize>,
}

impl Actor for TestActor {
    type Message = ();

    fn handle(&mut self, _system: &mut System, _id: Id, _message: ()) -> Result<After, Error> {
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(After::Stop)
    }
}
