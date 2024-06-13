use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

// Implementation of [`Deref`] and [`DerefMut`] enable the [`Guard`] pattern to
// be used here, rather than exposing an `pub unsafe fn unlock(...)` interface.
impl<T> Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Existence of the guard implies we have an exclusive lock, so this is
        // safe to do.
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Existence of the guard implies we have an exclusive lock, so this is
        // safe to do.
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        // When the guard is dropped, we should unlock
        self.lock.locked.store(false, Ordering::Relaxed)
    }
}

/// Implementation of a spinlock.
///
/// A spinlock means that a thread will continually retry unlocking the internal
/// structure rather than being put to sleep. The continual retry is useful where
/// a lock's lifetime would be very short and putting a thread to sleep and waking
/// it again may take even longer than a simple retry.
///
/// This builds from atomic operation principles to ensure that concurrent access
/// is safe, namely an [`AtomicBool`].
///
/// Extra details:
/// https://stackoverflow.com/questions/5869825/when-should-one-use-a-spinlock-instead-of-mutex
pub struct SpinLock<T> {
    pub data: UnsafeCell<T>,
    locked: AtomicBool,
}

// The use of [`UnsafeCell`] means we must promise to the compiler that this
// is okay to do.
unsafe impl<T> Sync for SpinLock<T> where T: Send {}

impl<T> SpinLock<T> {
    pub fn new(inner: T) -> Self {
        Self {
            data: UnsafeCell::new(inner),
            locked: AtomicBool::new(false),
        }
    }

    /// Acquire an exclusive mutable lock as a [`Guard`].
    ///
    /// The returned [`Guard`] enables unlocking the [`SpinLock`] when dropped.
    pub fn lock(&self) -> Guard<T> {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
        Guard { lock: self }
    }
}
