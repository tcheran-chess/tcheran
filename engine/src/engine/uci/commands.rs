use std::time::Duration;

use crate::engine::uci::UciMove;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Position {
    Fen(String),
    StartPos,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GoCmdArguments {
    pub ponder: bool,
    pub wtime: Option<Duration>,
    pub btime: Option<Duration>,
    pub winc: Option<Duration>,
    pub binc: Option<Duration>,
    pub movestogo: Option<u32>,
    pub depth: Option<u8>,
    pub nodes: Option<u32>,
    pub movetime: Option<Duration>,
    pub infinite: bool,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
    Go(GoCmdArguments),
    Stop,
    PonderHit,

    // OpenBench UCI commands
    Bench,
    BenchNodes,

    // Extra debug UCI commands
    PrintPosition,
    Perft {
        depth: u8,
    },
    PerftDiv {
        depth: u8,
    },
    Move {
        moves: Vec<UciMove>,
    },
    Eval,

    Quit,
}
