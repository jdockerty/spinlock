use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
};

use atomic_wait::wake_one;

pub struct WriteGuard<'a, T> {
    inner: &'a RwLock<T>,
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        if self.inner.state.swap(0, Ordering::Release) == 2 {
            // Wake a single waiting thread, if any
            wake_one(&self.inner.state);
        }
    }
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner.value.get() }
    }
}

// DerefMut is implemented for WriteGuard because it requires exclusive access
// to the data. The same is NOT true for the ReadGuard
impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.value.get() }
    }
}

pub struct ReadGuard<'a, T> {
    inner: &'a RwLock<T>,
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.inner.value.get() }
    }
}

pub struct RwLock<T> {
    // Numbers of readers or `u32::MAX` when there is a writer lock
    state: AtomicU32,
    value: UnsafeCell<T>,
}

// We also include the "where Sync" here because multiple readers may have
// access to the underlying data and be shared across threads.
// NOTE: A writer is exclusive and does not have this requirement.
unsafe impl<T> Sync for RwLock<T> where T: Send + Sync {}

impl<T> RwLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> ReadGuard<'_, T> {}

    pub fn write(&mut self) {}
}
