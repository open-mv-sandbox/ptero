use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use anyhow::Error;
use heck::ToKebabCase;
use thunderdome::Index;
use tracing::{span, Level, Span};

use crate::{dynamic::AnyActor, ActorAddr, Start, System};

pub struct DataFactory {
    starter: Box<dyn AnyStarter>,
}

impl DataFactory {
    pub fn new<S>(data: S::Data) -> Self
    where
        S: Start + 'static,
    {
        let starter: Starter<S> = Starter {
            data,
            _s: PhantomData,
        };
        Self {
            starter: Box::new(starter),
        }
    }

    pub fn create_span(&self) -> Span {
        self.starter.create_span()
    }

    pub fn start(self, system: &mut System, id: Index) -> Result<Box<dyn AnyActor>, Error> {
        self.starter.start(system, id)
    }
}

trait AnyStarter {
    fn create_span(&self) -> Span;

    fn start(self: Box<Self>, system: &mut System, id: Index) -> Result<Box<dyn AnyActor>, Error>;
}

struct Starter<S: Start> {
    data: S::Data,
    _s: PhantomData<AtomicPtr<S>>,
}

impl<S> AnyStarter for Starter<S>
where
    S: Start + 'static,
{
    fn create_span(&self) -> Span {
        let result = std::any::type_name::<S>().split("::").last();
        let type_name = match result {
            Some(value) => value,
            None => "Unknown",
        };

        let type_name_kebab = type_name.to_kebab_case();
        let id = type_name_kebab.trim_end_matches("-actor");
        span!(Level::INFO, "actor", id)
    }

    fn start(self: Box<Self>, system: &mut System, id: Index) -> Result<Box<dyn AnyActor>, Error> {
        let addr = ActorAddr::from_id(id);
        let actor = S::start(system, addr, self.data)?;
        Ok(Box::new(actor))
    }
}
