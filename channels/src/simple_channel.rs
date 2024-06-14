use std::collections::VecDeque;
use std::sync::{Mutex, Condvar};

pub struct SimpleChannel<T> {
    queue: Mutex<VecDeque<T>>,
    ready: Condvar,
}

/// A simple channel implementation through the use of a [`Mutex`] and [`Condvar`].
///
/// There are no uses of atomic variables explicitly here and therefore no unsafe
/// code in our case. The compiler understands that the mutex is Send/Sync and
/// will appropriately wrap and protect the queue for concurrent access.
///
/// The conditional variable ([`Condvar`]) is used to cause the [`receive`] function
/// to be blocking. The thread will block until a message can be received.
///
/// This would class as an unbounded channel, there is nothing stopping those who
/// send into the channel from outpacing the receive call.
impl<T> SimpleChannel<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            ready: Condvar::new()
        }
    }

    pub fn send(&self, message: T) {
        self.queue.lock().unwrap().push_back(message);
        // Wake up the blocked thread which is doing the receive.
        self.ready.notify_one();
    }

    pub fn receive(&self) -> T {
        let mut q = self.queue.lock().unwrap();
        loop {
            if let Some(message) = q.pop_front() {
                return message;
            }
            // Atomically unlock the mutex and wait for notification through
            // the [`Condvar`].
            // This means our mutex isn't locked for the entire duration of a
            // blocking receive if there are no messages in the channel.
            q = self.ready.wait(q).unwrap();
        }
    }
}
