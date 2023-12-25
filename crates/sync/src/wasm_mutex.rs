#![cfg(target_arch = "wasm32")]
use std::{
    ops::{Deref, DerefMut},
    sync::{MutexGuard, PoisonError, RwLockReadGuard, RwLockWriteGuard, TryLockError},
};

pub struct Mutex<T> {
    inner: std::sync::Mutex<T>,
}

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self { inner: std::sync::Mutex::new(t) }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T> Deref for Mutex<T> {
    type Target = std::sync::Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Mutex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub struct RwLock<T> {
    inner: std::sync::RwLock<T>,
}

impl<T> Deref for RwLock<T> {
    type Target = std::sync::RwLock<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for RwLock<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> RwLock<T> {
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        match self.inner.try_read() {
            Ok(e) => Some(e),
            Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        match self.inner.try_write() {
            Ok(e) => Some(e),
            Err(TryLockError::Poisoned(e)) => Some(e.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.inner.write().unwrap_or_else(PoisonError::into_inner)
    }
}
