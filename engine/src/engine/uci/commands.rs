use crate::engine::{search::TimeControl, uci::UciMove};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Position {
    Fen(String),
    StartPos,
}

#[derive(Debug, Clone)]
pub enum UciCommand {
    // Canon UCI commands
    Uci,
    Debug(bool),
    IsReady,
    SetOption {
        name: String,
        value: String,
    },
    UciNewGame,
    Position {
        position: Position,
        moves: Vec<UciMove>,
    },
    Go {
        time_control: TimeControl,
    },
    Stop,

    // OpenBench UCI commands
    Bench,
    BenchNodes,
    GenFens {
        n: u64,
        seed: u64,
        book: String,
        dfrc: bool,
    },

    // Extra debug UCI commands
    PrintPosition,
    Perft {
        depth: u8,
    },
    PerftDiv {
        depth: u8,
    },
    Move {
        moves: Vec<String>,
    },
    Eval,

    Spsa,

    Quit,
}
