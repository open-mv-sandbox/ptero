use std::{marker::PhantomData, sync::atomic::AtomicPtr};

use anyhow::Error;
use heck::ToKebabCase;
use thunderdome::Index;
use tracing::{span, Level, Span};

use crate::{dynamic::AnyActor, ActorAddrF, StartF, System};

pub trait AnyFactory {
    fn create_span(&self) -> Span;

    fn start(self: Box<Self>, system: &mut System, id: Index) -> Result<Box<dyn AnyActor>, Error>;
}

pub struct Factory<S: StartF> {
    data: S::Data,
    _s: PhantomData<AtomicPtr<S>>,
}

impl<S: StartF> Factory<S> {
    pub fn new(data: S::Data) -> Self {
        Self {
            data,
            _s: PhantomData,
        }
    }
}

impl<S> AnyFactory for Factory<S>
where
    S: StartF + 'static,
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
        let addr = ActorAddrF::from_id(id);
        let actor = S::start(system, addr, self.data)?;
        Ok(Box::new(actor))
    }
}
