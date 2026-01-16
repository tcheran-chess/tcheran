use std::time::{Duration, Instant};

use super::commands::UciCommand;
use crate::{
    chess::{
        piece::PromotionPieceKind,
        player::Player,
        square::{File, Rank, Square},
    },
    engine::{
        search::{Clocks, TimeControl},
        uci::{UciMove, commands::Position},
    },
};

fn boolean(input: &str) -> Result<bool, ()> {
    Ok(match input {
        "on" => true,
        "off" => false,
        _ => return Err(()),
    })
}

fn uci_square(input: &str) -> Result<Square, ()> {
    if input.len() != 2 {
        return Err(());
    }

    let mut chars = input.chars();
    let file = chars.next().unwrap();
    let rank = chars.next().unwrap();

    let file = match file {
        'a' => File::A,
        'b' => File::B,
        'c' => File::C,
        'd' => File::D,
        'e' => File::E,
        'f' => File::F,
        'g' => File::G,
        'h' => File::H,
        _ => return Err(()),
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
        _ => return Err(()),
    };

    Ok(Square::from_file_and_rank(file, rank))
}

fn uci_promotion(input: &str) -> Result<PromotionPieceKind, ()> {
    Ok(match input {
        "n" => PromotionPieceKind::Knight,
        "b" => PromotionPieceKind::Bishop,
        "r" => PromotionPieceKind::Rook,
        "q" => PromotionPieceKind::Queen,
        _ => return Err(()),
    })
}

#[expect(clippy::result_unit_err, reason = "No need for a better error message when this fails")]
pub fn uci_move(input: &str) -> Result<UciMove, ()> {
    Ok(match input.len() {
        len @ 4..=5 => {
            let src = input.get(0..=1).map_or(Err(()), uci_square)?;
            let dst = input.get(2..=3).map_or(Err(()), uci_square)?;

            let promotion = if len == 5 {
                let p = input.get(4..=4).unwrap();
                Some(uci_promotion(p)?)
            } else {
                None
            };

            UciMove {
                src,
                dst,
                promotion,
            }
        }
        _ => return Err(()),
    })
}

fn no_args_command(command: UciCommand, args: &[&str]) -> Result<UciCommand, ()> {
    if !args.is_empty() {
        return Err(());
    }

    Ok(command)
}

fn cmd_debug(args: &[&str]) -> Result<UciCommand, ()> {
    if args.len() != 1 {
        return Err(());
    }

    let onoff = args[0];
    let onoff = boolean(onoff)?;

    Ok(UciCommand::Debug(onoff))
}

// Note that we intentionally don't
fn cmd_setoption(args: &[&str]) -> Result<UciCommand, ()> {
    if args.is_empty() {
        return Err(());
    }

    // The first argument must be 'name'
    let name_arg = args[0];
    if name_arg != "name" {
        return Err(());
    }

    // All the tokens between 'name' and 'value' are the name
    let value_idx = args.iter().position(|&t| t == "value");
    let Some(value_idx) = value_idx else {
        return Err(());
    };

    let name_tokens = &args[1..value_idx];
    let value_tokens = &args[value_idx + 1..];

    if name_tokens.is_empty() || value_tokens.is_empty() {
        return Err(());
    }

    // Note that args containing multiple spaces between tokens will be reconstructed incorrectly here,
    // but we shouldn't ever have examples of that in practice so it's fine.
    Ok(UciCommand::SetOption {
        name: name_tokens.join(" "),
        value: value_tokens.join(" "),
    })
}

fn parse_moves(moves: &[&str]) -> Result<Vec<UciMove>, ()> {
    let moves = moves
        .iter()
        .map(|m| uci_move(m))
        .collect::<Result<Vec<UciMove>, ()>>()?;

    Ok(moves)
}

fn cmd_position(args: &[&str]) -> Result<UciCommand, ()> {
    if args.is_empty() {
        return Err(());
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
        _ => Err(()),
    }
}

fn parse_duration(n: &str) -> Result<Duration, ()> {
    let millis = n
        .parse::<i64>()
        .map_err(|_| ())?
        .max(0)
        .try_into()
        .map_err(|_| ())?;
    Ok(Duration::from_millis(millis))
}

fn cmd_go(args: &[&str]) -> Result<UciCommand, ()> {
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
            "wtime" => clocks.clocks[Player::White] = Some(parse_duration(args.next().ok_or(())?)?),
            "btime" => clocks.clocks[Player::Black] = Some(parse_duration(args.next().ok_or(())?)?),
            "winc" => {
                clocks.increments[Player::White] = Some(parse_duration(args.next().ok_or(())?)?);
            }
            "binc" => {
                clocks.increments[Player::Black] = Some(parse_duration(args.next().ok_or(())?)?);
            }
            "movestogo" => {
                clocks.moves_to_go = Some(args.next().ok_or(())?.parse().map_err(|_| ())?);
            }
            "movetime" => movetime = Some(parse_duration(args.next().ok_or(())?)?),
            "depth" => depth = Some(args.next().ok_or(())?.parse().map_err(|_| ())?),
            "nodes" => nodes = Some(args.next().ok_or(())?.parse::<u64>().map_err(|_| ())?),
            _ => return Err(()),
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
                // When doing datagen, 'go nodes n' means soft nodes n,
                // hard nodes n * f
                if cfg!(feature = "datagen") {
                    const DATAGEN_HARD_NODES_FACTOR: u64 = 10;

                    TimeControl::Nodes {
                        soft: Some(nodes),
                        hard: Some(nodes * DATAGEN_HARD_NODES_FACTOR),
                    }
                } else {
                    TimeControl::Nodes {
                        soft: None,
                        hard: Some(nodes),
                    }
                }
            } else if infinite {
                TimeControl::Infinite
            } else {
                unreachable!()
            }
        }
        _ => return Err(()),
    };

    Ok(UciCommand::Go { time_control })
}

fn cmd_move(args: &[&str]) -> Result<UciCommand, ()> {
    if args.is_empty() {
        return Err(());
    }

    Ok(UciCommand::Move {
        moves: args.iter().map(ToString::to_string).collect(),
    })
}

fn cmd_perft(args: &[&str]) -> Result<UciCommand, ()> {
    if args.len() != 1 {
        return Err(());
    }

    let depth = args[0].parse::<u8>().map_err(|_| ())?;
    Ok(UciCommand::Perft { depth })
}

fn cmd_perft_div(args: &[&str]) -> Result<UciCommand, ()> {
    if args.len() != 1 {
        return Err(());
    }

    let depth = args[0].parse::<u8>().map_err(|_| ())?;
    Ok(UciCommand::PerftDiv { depth })
}

fn cmd_genfens(args: &[&str]) -> Result<UciCommand, ()> {
    let mut args = args.iter();

    let n = args.next().ok_or(())?.parse::<u64>().map_err(|_| ())?;

    let mut seed: Option<u64> = None;
    let mut book: Option<String> = None;

    while let Some(&arg) = args.next() {
        match arg {
            "seed" => seed = Some(args.next().ok_or(())?.parse::<u64>().map_err(|_| ())?),
            "book" => book = Some(args.next().ok_or(())?.to_string()),
            _ => return Err(()),
        }
    }

    Ok(UciCommand::GenFens {
        n,
        seed: seed.ok_or(())?,
        book: book.ok_or(())?,
    })
}

#[expect(clippy::result_unit_err, reason = "Improved error reporting is planned")]
pub fn parse(input: &str) -> Result<UciCommand, ()> {
    let tokens = input.split_whitespace().collect::<Vec<&str>>();
    if tokens.is_empty() {
        return Err(());
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
        "genfens" => cmd_genfens(args),

        "pos" => no_args_command(UciCommand::PrintPosition, args),
        "move" => cmd_move(args),
        "perft" => cmd_perft(args),
        "perftdiv" => cmd_perft_div(args),
        "eval" => no_args_command(UciCommand::Eval, args),

        "spsa" => no_args_command(UciCommand::Spsa, args),

        "quit" => no_args_command(UciCommand::Quit, args),
        _ => Err(()),
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
