use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
};

/// Implementation of a channel through unsafe mechanisms.
///
/// This is problematic because the user can emerge panics through simple behaviour,
/// i.e. simply sending another message. A much better approach is to ensure that
/// the compiler can check correct usage and stop well before it can be mis-used.
/// This can be achieved through types. See `safe_oneshot.rs` for details.
pub struct UnsafeOneshotChannel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    in_use: AtomicBool,
    ready: AtomicBool,
}

unsafe impl<T> Sync for UnsafeOneshotChannel<T> where T: Send {}

impl<T> UnsafeOneshotChannel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            in_use: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    /// Send a message to the channel
    ///
    /// Panics:
    /// If called more than once
    pub fn send(&self, message: T) {
        if self.in_use.swap(true, Ordering::Relaxed) {
            panic!("Cannot send more than 1 message");
        }
        unsafe {
            (*self.message.get()).write(message);
        }
        self.ready.store(true, Ordering::Release);
    }

    /// Panics:
    /// When no messages are ready, use `is_ready()` first to check
    pub fn receive(&self) -> T {
        if !self.ready.swap(false, Ordering::Acquire) {
            panic!("Message not ready!");
        }
        unsafe { (*self.message.get()).assume_init_read() }
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }
}

impl<T> Drop for UnsafeOneshotChannel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}
