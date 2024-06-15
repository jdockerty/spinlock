use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{fence, AtomicUsize, Ordering};
use std::usize;

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Self {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(0),
                data,
            }))),
        }
    }

    pub fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    // arc: &mut Self is used here so that it must be called as Arc::get_mut(&mut value)
    // to avoid ambiguity with other methods on the underlying data (T).
    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Ordering::Relaxed) == 1 {
            fence(Ordering::Acquire);
            // Nothing else can access the Arc here, there is only a single
            // reference so this is safe to do
            unsafe { Some(&mut arc.ptr.as_mut().data) }
        } else {
            None
        }
    }
}

// Implement [`Deref`] so that the Arc transparently behaves like a reference to T.
// We cannot implement DerefMut here because Arc is shared ownership, not exclusive
// ownership. If we have DerefMut here, the structure could be altered by another
// referenced Arc.
impl<T> Deref for Arc<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data().data
    }
}

// Clone provides the same data pointer, but we atomically increment the reference count.
impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().ref_count.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Ordering::Release) == 1 {
            fence(Ordering::Acquire);
            // from_raw reclaims exclusive ownership so that we can drop the full
            // structure. We can only do this knowing we have the final reference.
            unsafe { drop(Box::from_raw(self.ptr.as_ptr())) }
        }
    }
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}
