pub mod defaults {
    pub const HASH_SIZE: usize = 16;
    pub const THREADS: usize = 1;
    pub const MOVE_OVERHEAD: usize = 0;
    pub const SYZYGY_PATH: Option<String> = None;
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub hash_size: usize,
    pub threads: usize,

    // Account for the possibility that there's some overhead making the move
    // e.g. sending the best move over the internet.
    pub move_overhead: usize,
    pub syzygy_path: Option<String>,
}

impl EngineOptions {
    pub const DEFAULT: Self = Self {
        hash_size: defaults::HASH_SIZE,
        threads: defaults::THREADS,
        move_overhead: defaults::MOVE_OVERHEAD,
        syzygy_path: defaults::SYZYGY_PATH,
    };
}
