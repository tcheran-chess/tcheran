use std::sync::atomic::{AtomicU64, Ordering};

pub struct BufferedAtomicU64<'s> {
    buffer: u64,
    local: u64,
    global: &'s AtomicU64,
}

impl<'s> BufferedAtomicU64<'s> {
    const BUFFER_SIZE: u64 = 4096;

    pub fn new(global: &'s AtomicU64) -> Self {
        Self {
            buffer: 0,
            local: 0,
            global,
        }
    }

    pub fn get(&self) -> u64 {
        self.local + self.buffer
    }

    // Will be missing buffer counts from instances that have not synced to global
    pub fn get_global(&self) -> u64 {
        self.global.load(Ordering::Relaxed) + self.buffer
    }

    pub fn incr(&mut self) {
        self.buffer += 1;

        if self.buffer >= Self::BUFFER_SIZE {
            self.local += self.buffer;
            self.global.fetch_add(self.buffer, Ordering::Relaxed);
            self.buffer = 0;
        }
    }
}
