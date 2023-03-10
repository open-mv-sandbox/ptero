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
    apply: fn(Apply, F::Member<'_>) -> Result<(), Error>,
    _p: PhantomData<*const F>,
}

impl<F> Sender<F>
where
    F: Family,
{
    /// Create a new `Sender`, with a custom ID and apply function.
    ///
    /// Currently, rust's type system doesn't quite coerce types right when passing to this
    /// function. You can get the compiler to accept your apply function by adding `as _` to
    /// explicitly tell the type system to perform the coercion.
    pub fn new(id: Id, apply: fn(Apply, F::Member<'_>) -> Result<(), Error>) -> Sender<F> {
        Self {
            id,
            apply,
            _p: PhantomData,
        }
    }

    /// Create a new `Sender` for an actor.
    pub fn actor<A>(info: Info<A>) -> Sender<A::Family>
    where
        A: Handler<Family = F> + 'static,
    {
        Self::new(info.id(), apply::<A>)
    }

    /// Send a message to an actor, which will handle it in-place.
    pub fn send<'a>(self, system: &mut System, message: impl Into<F::Member<'a>>) {
        let message = message.into();
        let apply = Apply {
            system,
            id: self.id,
        };
        let result = (self.apply)(apply, message);

        // TODO: Errors are intentionally ignored, but maybe it would be useful to have a unified way
        // to handle sending errors of any kind (including over a network).
        match result {
            Ok(value) => value,
            Err(error) => {
                // TODO: What to do with this?
                event!(Level::WARN, "failed to handle message\n{:?}", error);
            }
        }
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

/// Context data for apply.
pub struct Apply<'a> {
    system: &'a mut System,
    id: Id,
}

pub fn apply<A: Handler + 'static>(
    apply: Apply,
    message: <A::Family as Family>::Member<'_>,
) -> Result<(), Error> {
    let Apply { system, id } = apply;

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
