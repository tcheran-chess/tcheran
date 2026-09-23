use std::time::{Duration, Instant};

use super::commands::UciCommand;
use crate::{
    chess::prelude::*,
    engine::{
        search::{Clocks, TimeControl, types::Depth},
        uci::{UciMove, commands::Position},
        util::chars::StrParsingExtensions,
    },
};

fn boolean(input: &str) -> Result<bool, String> {
    Ok(match input {
        "on" => true,
        "off" => false,
        _ => return Err(format!("unknown boolean value: {input}")),
    })
}

fn uci_square(input: &str) -> Result<Square, String> {
    let Some([file, rank]) = input.as_char_array() else {
        return Err(format!("expected uci square, got {input}"));
    };

    let file = match file {
        'a' => File::A,
        'b' => File::B,
        'c' => File::C,
        'd' => File::D,
        'e' => File::E,
        'f' => File::F,
        'g' => File::G,
        'h' => File::H,
        _ => return Err(format!("invalid file: {file}")),
    };

    let rank = match rank {
        '1' => Rank::R1,
        '2' => Rank::R2,
        '3' => Rank::R3,
        '4' => Rank::R4,
        '5' => Rank::R5,
        '6' => Rank::R6,
        '7' => Rank::R7,
        '8' => Rank::R8,
        _ => return Err(format!("invalid rank: {rank}")),
    };

    Ok(Square::from_file_and_rank(file, rank))
}

fn uci_promotion(input: &str) -> Result<PromotionPieceKind, String> {
    Ok(match input {
        "n" => PromotionPieceKind::Knight,
        "b" => PromotionPieceKind::Bishop,
        "r" => PromotionPieceKind::Rook,
        "q" => PromotionPieceKind::Queen,
        _ => return Err(format!("invalid promotion piece: {input}")),
    })
}

pub fn uci_move(input: &str) -> Result<UciMove, String> {
    Ok(match input.len() {
        len @ 4..=5 => {
            let from = input
                .get(0..=1)
                .map_or_else(|| Err(format!("invalid uci move: {input}")), uci_square)?;
            let to = input
                .get(2..=3)
                .map_or_else(|| Err(format!("invalid uci move: {input}")), uci_square)?;

            let promotion = if len == 5 {
                let p = input.get(4..=4).unwrap();
                Some(uci_promotion(p)?)
            } else {
                None
            };

            UciMove {
                from,
                to,
                promotion,
            }
        }
        _ => return Err(format!("expected uci move, got {input}")),
    })
}

fn no_args_command(command: UciCommand, args: &[&str]) -> Result<UciCommand, String> {
    if !args.is_empty() {
        return Err("no arguments expected".to_string());
    }

    Ok(command)
}

fn cmd_debug(args: &[&str]) -> Result<UciCommand, String> {
    let &[onoff] = args else {
        return Err("invalid number of arguments".to_string());
    };

    let onoff = boolean(onoff)?;

    Ok(UciCommand::Debug(onoff))
}

fn cmd_setoption(args: &[&str]) -> Result<UciCommand, String> {
    let &[name_token, name_arg, value_token, value_arg] = args else {
        return Err("invalid number of arguments".to_string());
    };

    if name_token != "name" {
        return Err(format!("expected 'name', got {name_token}"));
    }

    if value_token != "value" {
        return Err(format!("expected 'value', got {value_token}"));
    }

    Ok(UciCommand::SetOption {
        name: name_arg.to_string(),
        value: value_arg.to_string(),
    })
}

fn parse_moves(moves: &[&str]) -> Result<Vec<UciMove>, String> {
    let moves = moves
        .iter()
        .map(|m| uci_move(m))
        .collect::<Result<Vec<UciMove>, String>>()?;

    Ok(moves)
}

fn cmd_position(args: &[&str]) -> Result<UciCommand, String> {
    if args.is_empty() {
        return Err("invalid number of arguments".to_string());
    }

    let mode = args[0];
    let rest = &args[1..];

    match mode {
        "startpos" => {
            let moves_token_idx = rest.iter().position(|&t| t == "moves");

            let moves = match moves_token_idx {
                Some(moves_token_idx) => parse_moves(&rest[moves_token_idx + 1..])?,
                None => Vec::new(),
            };

            Ok(UciCommand::Position {
                position: Position::StartPos,
                moves,
            })
        }
        "fen" => {
            let moves_token_idx = rest.iter().position(|&t| t == "moves");

            let fen = &rest[0..moves_token_idx.unwrap_or(rest.len())];
            let fen = fen.join(" ");

            let moves = match moves_token_idx {
                Some(moves_token_idx) => parse_moves(&rest[moves_token_idx + 1..])?,
                None => Vec::new(),
            };

            Ok(UciCommand::Position {
                position: Position::Fen(fen),
                moves,
            })
        }
        _ => Err(format!("expected 'startpos' or 'fen', got {mode}")),
    }
}

fn parse_duration(n: &str) -> Result<Duration, String> {
    let millis = n
        .parse::<i64>()
        .map_err(|_| format!("expected duration, got {n}"))?
        .max(0)
        .try_into()
        .map_err(|_| format!("expected duration, got {n}"))?;
    Ok(Duration::from_millis(millis))
}

fn cmd_go(args: &[&str]) -> Result<UciCommand, String> {
    let mut infinite = false;

    // Capture the start time as close as possible to when we parse the command to avoid excluding
    // search setup overhead from our time - see https://github.com/AndyGrant/Ethereal/issues/214
    let start_time = Instant::now();

    let mut clocks = Clocks {
        clocks: [None; Player::N],
        increments: [None; Player::N],
        moves_to_go: None,
    };

    let mut movetime = None;
    let mut depth = None;
    let mut nodes = None;

    let mut args = args.iter();
    while let Some(&arg) = args.next() {
        match arg {
            "infinite" => infinite = true,
            "wtime" => {
                clocks.clocks[White] =
                    Some(parse_duration(args.next().ok_or("expected duration for wtime")?)?);
            }
            "btime" => {
                clocks.clocks[Black] =
                    Some(parse_duration(args.next().ok_or("expected duration for btime")?)?);
            }
            "winc" => {
                clocks.increments[White] =
                    Some(parse_duration(args.next().ok_or("expected duration for winc")?)?);
            }
            "binc" => {
                clocks.increments[Black] =
                    Some(parse_duration(args.next().ok_or("expected duration for binc")?)?);
            }
            "movestogo" => {
                clocks.moves_to_go = Some(
                    args.next()
                        .ok_or("expected movestogo")?
                        .parse()
                        .map_err(|_| "invalid movestogo".to_string())?,
                );
            }
            "movetime" => {
                movetime =
                    Some(parse_duration(args.next().ok_or("expected duration for movetime")?)?);
            }
            "depth" => {
                depth = Some(
                    args.next()
                        .ok_or("expected depth")?
                        .parse::<u8>()
                        .map(Depth)
                        .map_err(|_| "invalid depth".to_string())?,
                );
            }
            "nodes" => {
                nodes = Some(
                    args.next()
                        .ok_or("expected nodes")?
                        .parse::<u64>()
                        .map_err(|_| "invalid nodes")?,
                );
            }
            _ => return Err(format!("unknown 'go' argument: {arg}")),
        }
    }

    let clocks_used = clocks.clocks.iter().any(Option::is_some)
        || clocks.increments.iter().any(Option::is_some)
        || clocks.moves_to_go.is_some();

    let time_control_types_used = [
        clocks_used,
        infinite,
        movetime.is_some(),
        depth.is_some(),
        nodes.is_some(),
    ]
    .into_iter()
    .filter(|t| *t)
    .count();

    let time_control = match time_control_types_used {
        0 => TimeControl::Infinite,
        1 => {
            if clocks_used {
                TimeControl::Clocks { clocks, start_time }
            } else if let Some(movetime) = movetime {
                TimeControl::ExactTime {
                    time: movetime,
                    start_time,
                }
            } else if let Some(depth) = depth {
                TimeControl::Depth(depth)
            } else if let Some(nodes) = nodes {
                TimeControl::Nodes {
                    soft: None,
                    hard: Some(nodes),
                }
            } else if infinite {
                TimeControl::Infinite
            } else {
                unreachable!()
            }
        }
        _ => return Err("conflicting time control types".to_string()),
    };

    Ok(UciCommand::Go { time_control })
}

fn cmd_move(args: &[&str]) -> Result<UciCommand, String> {
    if args.is_empty() {
        return Err("invalid number of arguments".to_string());
    }

    Ok(UciCommand::Move {
        moves: args.iter().map(ToString::to_string).collect(),
    })
}

fn cmd_perft(args: &[&str]) -> Result<UciCommand, String> {
    let &[depth] = args else {
        return Err("invalid number of arguments".to_string());
    };

    let depth = depth
        .parse::<u8>()
        .map_err(|_| "invalid depth".to_string())?;

    Ok(UciCommand::Perft { depth })
}

fn cmd_perft_div(args: &[&str]) -> Result<UciCommand, String> {
    let &[depth] = args else {
        return Err("invalid number of arguments".to_string());
    };

    let depth = depth
        .parse::<u8>()
        .map_err(|_| "invalid depth".to_string())?;

    Ok(UciCommand::PerftDiv { depth })
}

fn cmd_genfens(args: &[&str]) -> Result<UciCommand, String> {
    let mut args = args.iter();

    let n = args
        .next()
        .ok_or("expected number of FENs")?
        .parse::<u64>()
        .map_err(|_| "invalid number of FENs")?;

    let mut seed: Option<u64> = None;
    let mut book: Option<String> = None;
    let mut dfrc = false;

    while let Some(&arg) = args.next() {
        match arg {
            "seed" => {
                seed = Some(
                    args.next()
                        .ok_or("expected seed value")?
                        .parse::<u64>()
                        .map_err(|_| "invalid seed value")?,
                );
            }
            "book" => book = Some(args.next().ok_or("expected book")?.to_string()),
            "dfrc" => {
                dfrc = args
                    .next()
                    .ok_or("expected dfrc value")?
                    .parse::<bool>()
                    .map_err(|_| "invalid dfrc value")?;
            }
            _ => return Err(format!("unknown 'genfens' argument: {arg}")),
        }
    }

    Ok(UciCommand::GenFens {
        n,
        seed: seed.ok_or("expected 'seed'")?,
        book: book.ok_or("expected 'book'")?,
        dfrc,
    })
}

fn cmd_speedtest(args: &[&str]) -> Result<UciCommand, String> {
    let mut args = args.iter();

    let mut threads = None;
    let mut hash = None;
    let mut duration = None;

    while let Some(&arg) = args.next() {
        match arg {
            "threads" => {
                threads = Some(
                    args.next()
                        .ok_or("expected threads value")?
                        .parse::<u64>()
                        .map_err(|_| "invalid threads value")?,
                );
            }
            "hash" => {
                hash = Some(
                    args.next()
                        .ok_or("expected hash value")?
                        .parse::<u64>()
                        .map_err(|_| "invalid hash value")?,
                );
            }
            "duration" => {
                duration = Some(
                    args.next()
                        .ok_or("expected duration value")?
                        .parse::<u64>()
                        .map_err(|_| "invalid duration value")?,
                );
            }
            _ => return Err(format!("unknown 'speedtest' argument: {arg}")),
        }
    }

    Ok(UciCommand::Speedtest {
        threads,
        hash,
        duration,
    })
}

pub fn parse(input: &str) -> Result<UciCommand, String> {
    let tokens = input.split_whitespace().collect::<Vec<&str>>();
    if tokens.is_empty() {
        return Ok(UciCommand::Noop);
    }

    let command = tokens[0];
    let args = &tokens[1..];

    match command {
        "uci" => no_args_command(UciCommand::Uci, args),
        "debug" => cmd_debug(args),
        "isready" => no_args_command(UciCommand::IsReady, args),
        "setoption" => cmd_setoption(args),
        "ucinewgame" => no_args_command(UciCommand::UciNewGame, args),
        "position" => cmd_position(args),
        "go" => cmd_go(args),
        "stop" => no_args_command(UciCommand::Stop, args),

        "bench" => no_args_command(UciCommand::Bench, args),
        "benchnodes" => no_args_command(UciCommand::BenchNodes, args),
        "speedtest" => cmd_speedtest(args),
        "genfens" => cmd_genfens(args),

        "pos" => no_args_command(UciCommand::PrintPosition, args),
        "move" => cmd_move(args),
        "perft" => cmd_perft(args),
        "perftdiv" => cmd_perft_div(args),
        "eval" => no_args_command(UciCommand::Eval, args),

        "spsa" => no_args_command(UciCommand::Spsa, args),

        "quit" => no_args_command(UciCommand::Quit, args),
        _ => Err(format!("unknown command: {command}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_go_infinite() {
        assert!(parse("go infinite").is_ok());
    }

    #[test]
    fn test_uci() {
        let ml = parse("uci").unwrap();
        assert!(matches!(ml, UciCommand::Uci));
    }

    #[test]
    fn test_debug_on() {
        let ml = parse("debug    on").unwrap();
        assert!(matches!(ml, UciCommand::Debug(true)));
    }

    #[test]
    fn test_debug_off() {
        let ml = parse("debug off").unwrap();
        assert!(matches!(ml, UciCommand::Debug(false)));
    }

    #[test]
    fn test_debugon() {
        parse("debugon").expect_err("Should not parse 'debugon'");
    }

    #[test]
    fn test_debug_wrong_param() {
        let ml = parse("debug abc");
        assert!(ml.is_err());
    }

    #[test]
    fn test_debug_cutoff() {
        parse("debug    ontario").expect_err("Should not parse");
    }

    #[test]
    fn test_isready() {
        let ml = parse(" \tisready  ").unwrap();
        assert!(matches!(ml, UciCommand::IsReady));
    }

    #[test]
    fn test_position_fen() {
        let ml = parse("position fen 6r1/p2p4/3Ppk2/p1R2p2/8/3b4/1r6/4K3 b - - 5 45");
        assert!(ml.is_ok());
    }

    #[test]
    fn test_position_startpos_then_moves() {
        let ml = parse("position startpos moves e2e4 c7c5 g1f3 d7d6 f1b5 c8d7 b1c3");
        assert!(ml.is_ok());
    }

    #[test]
    fn test_position_fen_then_moves() {
        let result =
            parse("position fen 6r1/p2p4/3Ppk2/p1R2p2/8/3b4/1r6/4K3 b - - 5 45 moves a7a6 c1d1");
        assert!(result.is_ok());

        let components = result.unwrap();
        let UciCommand::Position { position, moves } = components else {
            panic!("Expected position command");
        };

        let Position::Fen(fen) = position else {
            panic!("Expected FEN position");
        };

        assert_eq!(fen, "6r1/p2p4/3Ppk2/p1R2p2/8/3b4/1r6/4K3 b - - 5 45");

        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0].notation(), "a7a6");
        assert_eq!(moves[1].notation(), "c1d1");
    }

    #[test]
    fn test_moves_with_promotion() {
        let result = parse("position fen 7k/P7/8/8/8/8/8/7K w - - 0 1 moves a7a8q");
        assert!(result.is_ok());

        let components = result.unwrap();
        let UciCommand::Position { moves, .. } = components else {
            panic!("Expected position command");
        };

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].notation(), "a7a8q");
    }
}
