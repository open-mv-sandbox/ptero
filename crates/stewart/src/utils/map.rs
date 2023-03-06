use anyhow::Error;

use crate::{AfterProcess, AfterReduce, System};

use crate::utils::{ActorT, AddrT};

/// Start actor that maps a value into another one.
pub fn start_map<F, A, B>(system: &mut System, map: F, target: AddrT<B>) -> Result<AddrT<A>, Error>
where
    F: FnMut(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    MapActor::start(system, map, target)
}

struct MapActor<F, A, B> {
    map: F,
    target: AddrT<B>,
    queue: Vec<A>,
}

impl<F, A, B> MapActor<F, A, B>
where
    F: FnMut(A) -> B + 'static,
    A: 'static,
    B: 'static,
{
    fn start(system: &mut System, map: F, target: AddrT<B>) -> Result<AddrT<A>, Error> {
        let addr = system.create("map");
        let actor = Self {
            map,
            target,
            queue: Vec::new(),
        };
        system.start(addr, actor)?;

        Ok(addr)
    }
}

impl<F, A, B> ActorT for MapActor<F, A, B>
where
    F: FnMut(A) -> B,
    A: 'static,
    B: 'static,
{
    type Message = A;

    fn reduce(&mut self, message: A) -> Result<AfterReduce, Error> {
        // TODO: Special 'reroute' option to shortcut the queue?
        self.queue.push(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        for message in self.queue.drain(..) {
            system.handle(self.target, (self.map)(message))?;
        }
        Ok(AfterProcess::Nothing)
    }
}
