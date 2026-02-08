use std::sync::{
    Arc,
    atomic::{
        AtomicU32,
        Ordering::{Acquire, Relaxed, Release},
    },
};

#[derive(Clone)]
pub struct NWaiter {
    n: u32,

    pub curr_n: Arc<AtomicU32>,
}

impl NWaiter {
    pub fn new(n: u32) -> Self {
        Self {
            n,

            curr_n: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn is_busy(&self) -> bool {
        self.curr_n.load(Relaxed) > 0
    }

    pub fn start(&self) {
        self.curr_n.store(self.n, Release);
    }

    pub fn decr(&self) {
        let r = self.curr_n.fetch_sub(1, Acquire);

        // The main thread may be waiting for other threads to finish
        // or the UCI thread may be waiting for all threads to finish.
        if r == 1 || r == 2 {
            atomic_wait::wake_all(&raw const *self.curr_n);
        }
    }

    pub fn wait_until_last(&self) {
        self.wait_until(1);
    }

    pub fn wait_until_finished(&self) {
        self.wait_until(0);
    }

    fn wait_until(&self, n: u32) {
        let mut curr_n = self.curr_n.load(Acquire);
        if curr_n == n {
            return;
        }

        while curr_n != n {
            atomic_wait::wait(&self.curr_n, curr_n);
            curr_n = self.curr_n.load(Acquire);
        }
    }
}
