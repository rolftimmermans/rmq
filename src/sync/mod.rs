#[cfg(loom)]
mod loom;

#[cfg(not(loom))]
mod loom {
    pub(crate) use parking_lot::{Mutex, MutexGuard, RwLock};
    pub(crate) use std::sync::{atomic, Arc};
}

pub(crate) use self::loom::*;
