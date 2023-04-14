use std::{
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Error};
use rstest::{fixture, rstest};
use stewart::{ActorId, Addr, StartError, State, System, SystemId, SystemOptions, World};
use tracing::{event, Level};
use tracing_test::traced_test;

#[rstest]
#[traced_test]
fn system_sends_message_to_actor(mut world: TestWorld) -> Result<(), Error> {
    // Send a message
    world.world.send(world.root.addr, ());
    world.world.run_until_idle()?;

    // Actor will now have removed itself, send again to make sure it doesn't crash
    world.world.send(world.root.addr, ());
    world.world.run_until_idle()?;

    // Make sure the first message was handled, but not the second one
    assert_eq!(world.root.count.load(Ordering::SeqCst), 1);

    Ok(())
}

#[rstest]
#[traced_test]
fn system_stops_actors(mut world: TestWorld) -> Result<(), Error> {
    // Send a message to the part
    world.world.send(world.root.addr, ());
    world.world.run_until_idle()?;

    // Actor will now have removed itself, try sending a message to the child
    world.world.send(world.child.addr, ());
    world.world.run_until_idle()?;

    // Make sure it wasn't handled
    assert_eq!(world.child.count.load(Ordering::SeqCst), 0);

    Ok(())
}

#[rstest]
#[traced_test]
fn system_removes_not_started() -> Result<(), Error> {
    let mut world = World::new();
    let system = world.register(SystemOptions::default(), TestActorSystem);

    let actor = world.create(system, None)?;

    // Process, this should remove the stale actor
    world.run_until_idle()?;

    // Make sure we can't start
    let result = world.start(actor, TestActor::default());
    if let Err(StartError::ActorNotFound) = result {
        event!(Level::INFO, "correct result");
    } else {
        assert!(false, "incorret result: {:?}", result);
    }

    Ok(())
}

#[fixture]
fn world() -> TestWorld {
    let mut world = World::new();
    let system = world.register(SystemOptions::default(), TestActorSystem);

    let root = create_actor(&mut world, system, None);
    let child = create_actor(&mut world, system, Some(root.id));

    TestWorld { world, root, child }
}

fn create_actor<'a>(world: &mut World, system: SystemId, parent: Option<ActorId>) -> ActorInfo {
    let actor = world.create(system, parent).unwrap();

    let instance = TestActor::default();
    let count = instance.count.clone();
    world.start(actor, instance).unwrap();

    let info = ActorInfo {
        id: actor,
        addr: Addr::new(actor),
        count,
    };

    info
}

struct TestWorld {
    world: World,
    root: ActorInfo,
    child: ActorInfo,
}

struct ActorInfo {
    id: ActorId,
    addr: Addr<()>,
    count: Rc<AtomicUsize>,
}

struct TestActorSystem;

impl System for TestActorSystem {
    type Instance = TestActor;
    type Message = ();

    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        while let Some((id, _)) = state.next() {
            let instance = state.get_mut(id).context("failed to get instance")?;

            instance.count.fetch_add(1, Ordering::SeqCst);
            world.stop(id)?;
        }

        Ok(())
    }
}

#[derive(Default)]
struct TestActor {
    count: Rc<AtomicUsize>,
}
