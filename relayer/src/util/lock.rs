use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub type RwArc<T> = Arc<RwLock<T>>;

pub trait LockExt<T> {
    fn new_lock(val: T) -> Self;

    fn acquire_read(&self) -> RwLockReadGuard<'_, T>;

    fn acquire_write(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T> LockExt<T> for Arc<RwLock<T>> {
    fn new_lock(val: T) -> Self {
        Arc::new(RwLock::new(val))
    }

    fn acquire_read(&self) -> RwLockReadGuard<'_, T> {
        self.read().expect("poisoned lock")
    }

    fn acquire_write(&self) -> RwLockWriteGuard<'_, T> {
        self.write().expect("poisoned lock")
    }
}
