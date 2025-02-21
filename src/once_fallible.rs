use std::sync::{
    atomic::{
        AtomicBool,
        Ordering::{Acquire, Release},
    },
    Mutex,
};

#[derive(Debug)]
pub struct OnceFallible {
    lock: Mutex<()>,
    done: AtomicBool,
}

impl OnceFallible {
    pub fn call_once_maybe<T>(&self, f: T) -> bool
    where
        T: FnOnce() -> bool,
    {
        let l = self.lock.try_lock();
        if l.is_ok() {
            if f() {
                self.done.store(true, Release);
                return true;
            }
        }
        false
    }

    pub fn is_completed(&self) -> bool {
        self.done.load(Acquire)
    }

    pub fn new() -> Self {
        Self {
            lock: Mutex::new(()),
            done: AtomicBool::new(false),
        }
    }
}
