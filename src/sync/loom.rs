#![cfg(loom)]

pub(crate) use loom::sync::atomic;
pub(crate) use loom::sync::Arc;

#[derive(Debug)]
pub(crate) struct Mutex<T> {
    mutex: loom::sync::Mutex<T>,
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self {
            mutex: loom::sync::Mutex::new(Default::default()),
        }
    }
}

impl<T> Mutex<T> {
    pub fn lock(&self) -> loom::sync::MutexGuard<'_, T> {
        self.mutex.lock().unwrap()
    }
}

#[derive(Debug)]
// https://github.com/tokio-rs/loom/pull/88
pub(crate) struct RwLock<T> {
    mutex: loom::sync::Mutex<T>,
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> Self {
        Self {
            mutex: loom::sync::RwLock::new(Default::default()),
        }
    }
}

impl<T> RwLock<T> {
    pub fn read(&self) -> loom::sync::MutexGuard<'_, T> {
        self.mutex.lock().unwrap()
    }

    pub fn write(&self) -> loom::sync::MutexGuard<'_, T> {
        self.mutex.lock().unwrap()
    }
}
