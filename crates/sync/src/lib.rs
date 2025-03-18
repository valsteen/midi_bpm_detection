#![allow(forbidden_lint_groups)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

mod serializable_atomic;
mod wasm_mutex;

use std::{
    mem,
    sync::{Arc, Weak},
};

#[cfg(not(target_arch = "wasm32"))]
pub use parking_lot::*;
pub use serializable_atomic::{ArcAtomicBool, ArcAtomicOptional};
#[cfg(target_arch = "wasm32")]
pub use wasm_mutex::{Mutex, RwLock};

#[derive(Debug)]
pub struct WouldBlock;

pub trait ArcRwLockExt<T> {
    fn get<R>(&self, closure: impl FnOnce(&T) -> R) -> R;
    fn get_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> R;
    fn set(&self, arg: T) -> T;

    fn try_get<R>(&self, closure: impl FnOnce(&T) -> R) -> Result<R, WouldBlock>;
    fn try_get_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Result<R, WouldBlock>;
    fn try_set(&self, arg: T) -> Result<T, WouldBlock>;
}

pub trait WeakRwLockOptionExt<T> {
    fn get_option<R>(&self, closure: impl FnOnce(&T) -> R) -> Option<R>;
    fn get_option_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Option<R>;

    fn try_get_option<R>(&self, closure: impl FnOnce(&T) -> R) -> Result<Option<R>, WouldBlock>;
    fn try_get_option_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Result<Option<R>, WouldBlock>;
    fn take(&self) -> Option<T>;
    fn set_option(&self, arg: T) -> Option<T>;
    fn try_take(&self) -> Result<Option<T>, WouldBlock>;
}

pub trait RwLockOptionExt<T>: WeakRwLockOptionExt<T> {}

pub type RwLockOption<T> = RwLock<Option<T>>;
pub type ArcRwLockOption<T> = Arc<RwLock<Option<T>>>;
pub type ArcRwLock<T> = Arc<RwLock<T>>;

pub type WeakRwLock<T> = Weak<RwLock<T>>;
pub type WeakRwLockOption<T> = Weak<RwLock<Option<T>>>;

impl<T> ArcRwLockExt<T> for RwLock<T> {
    fn get<R>(&self, closure: impl FnOnce(&T) -> R) -> R {
        let guard = RwLock::read(self);
        closure(&*guard)
    }

    fn get_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> R {
        let mut guard = RwLock::write(self);
        closure(&mut *guard)
    }

    fn set(&self, arg: T) -> T {
        let mut guard = RwLock::write(self);
        mem::replace(&mut *guard, arg)
    }

    fn try_get<R>(&self, closure: impl FnOnce(&T) -> R) -> Result<R, WouldBlock> {
        let guard = RwLock::try_read(self).ok_or(WouldBlock)?;
        Ok(closure(&*guard))
    }

    fn try_get_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Result<R, WouldBlock> {
        let mut guard = RwLock::try_write(self).ok_or(WouldBlock)?;
        Ok(closure(&mut *guard))
    }

    fn try_set(&self, arg: T) -> Result<T, WouldBlock> {
        let mut guard = RwLock::try_write(self).ok_or(WouldBlock)?;
        Ok(mem::replace(&mut *guard, arg))
    }
}

impl<T> WeakRwLockOptionExt<T> for WeakRwLockOption<T> {
    fn get_option<R>(&self, closure: impl FnOnce(&T) -> R) -> Option<R> {
        self.upgrade()?.get_option(closure)
    }

    fn get_option_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.upgrade()?.get_option_mut(closure)
    }

    fn try_get_option<R>(&self, closure: impl FnOnce(&T) -> R) -> Result<Option<R>, WouldBlock> {
        let Some(arc) = self.upgrade() else { return Ok(None) };
        arc.try_get_option(closure)
    }

    fn try_get_option_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Result<Option<R>, WouldBlock> {
        let Some(arc) = self.upgrade() else { return Ok(None) };
        arc.try_get_option_mut(closure)
    }

    fn take(&self) -> Option<T> {
        self.upgrade()?.take()
    }

    fn set_option(&self, arg: T) -> Option<T> {
        self.upgrade()?.set_option(arg)
    }

    fn try_take(&self) -> Result<Option<T>, WouldBlock> {
        let Some(arc) = self.upgrade() else { return Ok(None) };
        arc.try_take()
    }
}

impl<T> WeakRwLockOptionExt<T> for RwLockOption<T> {
    fn get_option<R>(&self, closure: impl FnOnce(&T) -> R) -> Option<R> {
        self.get(move |t| t.as_ref().map(closure))
    }

    fn get_option_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Option<R> {
        ArcRwLockExt::get_mut(self, move |t| t.as_mut().map(closure))
    }

    fn try_get_option<R>(&self, closure: impl FnOnce(&T) -> R) -> Result<Option<R>, WouldBlock> {
        self.try_get(|t| t.as_ref().map(closure))
    }

    fn try_get_option_mut<R>(&self, closure: impl FnOnce(&mut T) -> R) -> Result<Option<R>, WouldBlock> {
        self.try_get_mut(|t| t.as_mut().map(closure))
    }

    fn take(&self) -> Option<T> {
        self.set(None)
    }

    fn set_option(&self, arg: T) -> Option<T> {
        ArcRwLockExt::get_mut(self, |value| value.replace(arg))
    }

    fn try_take(&self) -> Result<Option<T>, WouldBlock> {
        self.try_set(None)
    }
}
impl<T> RwLockOptionExt<T> for RwLockOption<T> {}
