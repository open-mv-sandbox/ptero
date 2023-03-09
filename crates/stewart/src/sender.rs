use std::marker::PhantomData;

use anyhow::Error;
use family::Family;
use thunderdome::Index;
use tracing::{event, Level};

use crate::{Actor, After, Id, System};

/// Address for sending messages to an actor.
///
/// `Addr` is intentionally !Send + !Sync. In most cases sending an addr between threads is a
/// mistake, as it's only valid for one `System`, and `System` is !Send + !Sync.
pub struct Sender<F>
where
    F: Family,
{
    pub(crate) index: Index,
    apply: fn(&mut System, Id, F::Member<'_>),
    _p: PhantomData<*const F>,
}

impl<F> Sender<F>
where
    F: Family,
{
    pub(crate) fn new<A>(index: Index) -> Self
    where
        A: Actor<Family = F> + 'static,
    {
        Self {
            index,
            apply: apply_handle::<A>,
            _p: PhantomData,
        }
    }
}

impl<F> Sender<F>
where
    F: Family,
{
    pub fn send<'a>(self, system: &mut System, message: impl Into<F::Member<'a>>) {
        let message = message.into();
        (self.apply)(system, Id { index: self.index }, message)
    }
}

impl<F> Clone for Sender<F>
where
    F: Family,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<F> Copy for Sender<F> where F: Family {}

fn apply_handle<A: Actor + 'static>(
    system: &mut System,
    id: Id,
    message: <A::Family as Family>::Member<'_>,
) {
    let result = apply_handle_try::<A>(system, id, message);
    match result {
        Ok(value) => value,
        Err(error) => {
            // TODO: What to do with this?
            event!(Level::WARN, "failed to handle message\n{:?}", error);
        }
    }
}

fn apply_handle_try<A: Actor + 'static>(
    system: &mut System,
    id: Id,
    message: <A::Family as Family>::Member<'_>,
) -> Result<(), Error> {
    // Attempt to borrow the actor for handling
    let (span, mut actor) = system.borrow_actor::<A>(id)?;
    let _entry = span.enter();

    // Let the actor handle the message
    let result = actor.handle(system, message);

    // Handle the result
    let after = match result {
        Ok(value) => value,
        Err(error) => {
            // TODO: What to do with this?
            event!(Level::ERROR, "actor failed to reduce message\n{:?}", error);
            After::Nothing
        }
    };

    // Return the actor
    system.return_actor(id, actor, after)?;

    Ok(())
}
