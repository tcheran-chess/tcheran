mod fen_parser;
mod fen_writer;

pub use fen_parser::{ParseError, parse};
pub use fen_writer::write;

pub const START_POS: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
