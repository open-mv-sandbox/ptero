use std::collections::VecDeque;

/// Actor messages collection, passed to an actor on process for handling pending messages.
pub struct Messages<M> {
    queue: VecDeque<M>,
}

impl<M> Messages<M> {
    pub(crate) fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub(crate) fn enqueue(&mut self, message: M) {
        self.queue.push_back(message);
    }

    pub(crate) fn has_queued(&mut self) -> bool {
        !self.queue.is_empty()
    }

    /// Get the next pending message, or None if no messages are left.
    pub fn next(&mut self) -> Option<M> {
        self.queue.pop_front()
    }
}
