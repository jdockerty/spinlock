use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

/// Implementation of a channel through a safe mechanism.
///
/// The channel itself is now considered an internal implementation detail.
/// We expose the use of sending and receiving through types, ensuring that they
/// cannot be mis-used with help from the Rust compiler.
///
/// NOTE: This pattern is exactly how types are implemented in the Rust standard
/// library and other popular libraries.
struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let a = Arc::new(Channel {
        message: UnsafeCell::new(MaybeUninit::uninit()),
        ready: AtomicBool::new(false),
    });
    (
        Sender {
            channel: Arc::clone(&a),
        },
        Receiver { channel: a },
    )
}

pub struct Sender<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Sender<T> {
    // The use of `self` and not `&self` here stops this panicking because `send`
    // consumes and now owns the `Sender`, meaning it cannot be called multiple times.
    // Attempting to do so by a user will be caught by the compiler, removing
    // any possibility of user errors and panics, as is present in the unsafe
    // implementation.
    pub fn send(self, message: T) {
        unsafe { (*self.channel.message.get()).write(message) };
        self.channel.ready.store(true, Ordering::Release);
    }
}

pub struct Receiver<T> {
    channel: Arc<Channel<T>>,
}

impl<T> Receiver<T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Ordering::Relaxed)
    }
    pub fn receive(&self) -> T {
        if !self.channel.ready.swap(false, Ordering::Acquire) {
            panic!("No messages");
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}
