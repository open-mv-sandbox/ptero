use std::marker::PhantomData;

use anyhow::Error;
use family::{utils::FamilyT, Family};
use tracing::{event, Level};

use crate::{After, Id, Info, System};

use super::Handler;

/// Address for sending messages to an actor.
///
/// `Addr` is intentionally !Send + !Sync. In most cases sending an addr between threads is a
/// mistake, as it's only valid for one `System`, and `System` is !Send + !Sync.
pub struct Sender<F>
where
    F: Family,
{
    id: Id,
    apply: fn(&mut System, Id, F::Member<'_>),
    _p: PhantomData<*const F>,
}

impl<F> Sender<F>
where
    F: Family,
{
    pub fn new<A: Handler<Family = F>>(info: Info<A>) -> Sender<A::Family>
    where
        A: 'static,
    {
        Self {
            id: info.id(),
            // TODO: We can use a static marker trait with a function here for cheap mapping.
            // Passing the trait to the apply function will get us a unique combination function.
            apply: apply_handle::<A>,
            _p: PhantomData,
        }
    }

    /// Send a message to an actor, which will handle it in-place.
    ///
    /// TODO: Errors are intentionally ignored, but maybe it would be useful to have a unified way
    /// to handle sending errors of any kind (including over a network).
    pub fn send<'a>(self, system: &mut System, message: impl Into<F::Member<'a>>) {
        let message = message.into();
        (self.apply)(system, self.id, message)
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

fn apply_handle<A: Handler + 'static>(
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

fn apply_handle_try<A: Handler + 'static>(
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

/// Convenience alias for sender to a `HandlerT`.
pub type SenderT<T> = Sender<FamilyT<T>>;
