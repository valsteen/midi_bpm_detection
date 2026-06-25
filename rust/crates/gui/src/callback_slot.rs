use std::sync::{Arc, Weak};

use sync::Mutex;

/// GUI-owned callback slot shared with remotes through strong and weak handles.
pub(crate) type CallbackSlot<T> = Mutex<Option<Box<T>>>;
pub(crate) type ArcCallbackSlot<T> = Arc<CallbackSlot<T>>;
pub(crate) type WeakCallbackSlot<T> = Weak<CallbackSlot<T>>;
