use std::{
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
        if self.inner.state.swap(0, Ordering::Release) == 2 {
            // Wake a single waiting thread, if any
            wake_one(&self.inner.state);
        }
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
    // 0: unlocked
    // 1: locked with waiting threads
    // 2: locked with no waiting threads
    // This optimisation avoids unnecessary syscalls for waking waiting threads
    // by tracking when a wake is actually required.
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
        // If an err occurs on the swap, the mutex has been locked previously
        if self
            .state
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // The atomic swap to 2 here is because we know the mutex is locked
            // already, so we set the state to "multiple threads waiting"
            //
            // The return of anything but 0 (unlocked) will cause a wait on this
            // thread
            while self.state.swap(2, Ordering::Acquire) != 0 {
                // If we reach here, the lock has already been locked, so we should
                // sleep until unlocked by a wake
                wait(&self.state, 2);
            }
        }
        MutexGuard { inner: self }
    }
}
