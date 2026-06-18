use std::time::Duration;

pub mod defaults {
    use std::time::Duration;

    pub const HASH_SIZE: usize = 16;
    pub const THREADS: usize = 1;
    pub const MINIMAL: bool = false;
    pub const FRC: bool = false;

    pub const MOVE_OVERHEAD: Duration = Duration::from_millis(0);
    pub const SYZYGY_PATH: Option<String> = None;
    pub const SHOW_WDL: bool = true;

    pub const SOFT_NODES: bool = false;
    pub const SOFT_NODES_HARD_FACTOR: Option<usize> = None;
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    pub hash_size: usize,
    pub threads: usize,
    pub minimal: bool,
    pub frc: bool,

    // Account for the possibility that there's some overhead making the move
    // e.g. sending the best move over the internet.
    pub move_overhead: Duration,
    pub syzygy_path: Option<String>,

    pub soft_nodes: bool,
    pub soft_notes_hard_factor: Option<usize>,
}

impl EngineOptions {
    pub const DEFAULT: Self = Self {
        hash_size: defaults::HASH_SIZE,
        threads: defaults::THREADS,
        minimal: defaults::MINIMAL,
        frc: defaults::FRC,

        move_overhead: defaults::MOVE_OVERHEAD,
        syzygy_path: defaults::SYZYGY_PATH,

        soft_nodes: defaults::SOFT_NODES,
        soft_notes_hard_factor: defaults::SOFT_NODES_HARD_FACTOR,
    };
}
