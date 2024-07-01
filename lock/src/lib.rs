use std::{
    borrow::BorrowMut,
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

use atomic_wait::{wait, wake_one};

pub struct MutexGuard<'a, T> {
    inner: &'a Mutex<T>,
}

impl<T> Drop for MutexGuard<'_, T> {

    fn drop(&mut self) {
        // State back to unlocked
        self.inner.state.store(0, Ordering::Release);
        // Wake a single waiting thread, if any
        wake_one(&self.inner.state);
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.value.get() }
    }
}

pub struct Mutex<T> {
    // 0 for unlocked
    // 1 for locked
    state: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        // The `wait` can return spuriously, so we use a while loop here to
        // check the condition again for a guarantee
        while self.state.swap(1, Ordering::Acquire) == 1 {
            // If we reach here, the lock has already been locked, so we should
            // sleep until unlocked by a wake
            wait(&self.state, 1);
        }
        MutexGuard { inner: self }
    }
}
